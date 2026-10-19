#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use darling::{
    FromDeriveInput, FromField, FromMeta, FromVariant,
    ast::{Data, Fields, Style},
    util::Flag,
};
use proc_macro::TokenStream;
use proc_macro2::{Literal, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    DeriveInput, Expr, ExprLit, ExprUnary, GenericParam, Index, Lifetime, LifetimeParam, Lit,
    LitInt, LitStr, Member, Path, Type, UnOp,
    visit::{self, Visit},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromMeta)]
#[darling(rename_all = "kebab-case")]
enum Profile {
    Rfc,
    PrefPlus,
    Deterministic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Int(i128),
    Text(String),
}

impl FromMeta for Key {
    fn from_value(value: &Lit) -> darling::Result<Self> {
        match value {
            Lit::Str(value) => Ok(Self::Text(value.value())),
            Lit::Int(value) => int_key(value),
            _ => Err(darling::Error::custom(
                "key must be a string or integer literal",
            )),
        }
    }

    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        match expr {
            Expr::Lit(lit) => Self::from_value(&lit.lit),
            Expr::Unary(ExprUnary {
                op: UnOp::Neg(_),
                expr,
                ..
            }) => match &**expr {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(value),
                    ..
                }) => {
                    let value = -value.base10_parse::<i128>()?;
                    if value >= i64::MIN as i128 {
                        Ok(Self::Int(value))
                    } else {
                        Err(darling::Error::custom("negative key is below i64::MIN"))
                    }
                }
                _ => Err(darling::Error::custom(
                    "negative key must be an integer literal",
                )),
            },
            Expr::Group(group) => Self::from_expr(&group.expr),
            _ => Err(darling::Error::custom(
                "key must be a string or integer literal",
            )),
        }
    }
}

fn int_key(value: &LitInt) -> darling::Result<Key> {
    let value = value.base10_parse::<i128>()?;
    if (i64::MIN as i128..=u64::MAX as i128).contains(&value) {
        Ok(Key::Int(value))
    } else {
        Err(darling::Error::custom("integer key must fit in u64 or i64"))
    }
}

#[derive(Debug, Clone)]
enum DefaultValue {
    Flag,
    Expr(Expr),
}

impl FromMeta for DefaultValue {
    fn from_word() -> darling::Result<Self> {
        Ok(Self::Flag)
    }

    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        Ok(Self::Expr(expr.clone()))
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(cbor), supports(struct_any, enum_any))]
struct CborAttrs {
    ident: syn::Ident,
    generics: syn::Generics,
    data: Data<VariantAttrs, FieldAttrs>,
    #[darling(default)]
    tag: Option<u64>,
    #[darling(default)]
    profile: Option<Profile>,
    map: Flag,
    array: Flag,
}

#[derive(Debug, FromField)]
#[darling(attributes(cbor))]
struct FieldAttrs {
    ident: Option<syn::Ident>,
    ty: Type,
    #[darling(default)]
    key: Option<Key>,
    #[darling(default)]
    tag: Option<u64>,
    simple_value: Flag,
    #[darling(default)]
    with: Option<Path>,
    #[darling(default)]
    default: Option<DefaultValue>,
    skip: Flag,
    indefinite: Flag,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(cbor))]
struct VariantAttrs {
    ident: syn::Ident,
    fields: Fields<FieldAttrs>,
    #[darling(default)]
    key: Option<Key>,
    #[darling(default)]
    tag: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Serialize,
    Deserialize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Map,
    Array,
    Newtype,
    Unit,
}

struct FieldInfo<'a> {
    index: usize,
    attrs: &'a FieldAttrs,
    member: Member,
    ty: &'a Type,
    name: LitStr,
}

impl<'a> FieldInfo<'a> {
    fn slot(&self) -> syn::Ident {
        format_ident!("__seabored_field_{}", self.index)
    }

    fn key(&self) -> Key {
        self.attrs.key.clone().unwrap_or_else(|| {
            Key::Text(
                self.attrs
                    .ident
                    .as_ref()
                    .expect("map fields have an identifier")
                    .to_string(),
            )
        })
    }

    fn default_expr(&self) -> TokenStream2 {
        match &self.attrs.default {
            Some(DefaultValue::Expr(expr)) => quote!(#expr),
            Some(DefaultValue::Flag) | None => quote!(::core::default::Default::default()),
        }
    }
}

#[proc_macro_derive(CborSerialize, attributes(cbor))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    expand(input, Direction::Serialize)
}

#[proc_macro_derive(CborDeserialize, attributes(cbor))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    expand(input, Direction::Deserialize)
}

fn expand(input: TokenStream, direction: Direction) -> TokenStream {
    let input = match syn::parse::<DeriveInput>(input) {
        Ok(input) => input,
        Err(error) => return error.to_compile_error().into(),
    };

    let attrs = match CborAttrs::from_derive_input(&input) {
        Ok(attrs) => attrs,
        Err(error) => return error.write_errors().into(),
    };

    if let Err(error) = validate(&attrs) {
        return error.write_errors().into();
    }

    match generate(&attrs, direction) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn struct_form(attrs: &CborAttrs) -> syn::Result<(Form, bool)> {
    let fields = match &attrs.data {
        Data::Struct(fields) => fields,
        Data::Enum(_) => {
            return Err(syn::Error::new_spanned(
                &attrs.ident,
                "enums are implemented in phase 4",
            ));
        }
    };

    Ok(match fields.style {
        Style::Unit => (Form::Unit, false),
        Style::Tuple if fields.fields.len() == 1 => (Form::Newtype, false),
        Style::Tuple => (Form::Array, false),
        Style::Struct => {
            if attrs.array.is_present() {
                (Form::Array, true)
            } else {
                (Form::Map, true)
            }
        }
    })
}

fn field_infos(fields: &[FieldAttrs]) -> Vec<FieldInfo<'_>> {
    fields
        .iter()
        .enumerate()
        .map(|(index, attrs)| {
            let member = match &attrs.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index::from(index)),
            };
            let name = LitStr::new(
                &attrs
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| index.to_string()),
                Span::call_site(),
            );
            FieldInfo {
                index,
                attrs,
                member,
                ty: &attrs.ty,
                name,
            }
        })
        .collect()
}

fn struct_fields(attrs: &CborAttrs) -> syn::Result<Vec<FieldInfo<'_>>> {
    match &attrs.data {
        Data::Struct(fields) => Ok(field_infos(&fields.fields)),
        Data::Enum(_) => Err(syn::Error::new_spanned(
            &attrs.ident,
            "enums are implemented in phase 4",
        )),
    }
}

struct VariantInfo<'a> {
    attrs: &'a VariantAttrs,
    form: Form,
    named: bool,
    fields: Vec<FieldInfo<'a>>,
    key: Key,
}

fn enum_variants(attrs: &CborAttrs) -> syn::Result<Vec<VariantInfo<'_>>> {
    let variants = match &attrs.data {
        Data::Enum(variants) => variants,
        Data::Struct(_) => {
            return Err(syn::Error::new_spanned(
                &attrs.ident,
                "struct codegen reached enum_variants",
            ));
        }
    };
    Ok(variants
        .iter()
        .map(|variant| {
            let (form, named) = match variant.fields.style {
                Style::Unit => (Form::Unit, false),
                Style::Tuple if variant.fields.fields.len() == 1 => (Form::Newtype, false),
                Style::Tuple => (Form::Array, false),
                Style::Struct => (Form::Map, true),
            };
            let fields = field_infos(&variant.fields.fields);
            VariantInfo {
                attrs: variant,
                form,
                named,
                fields,
                key: variant
                    .key
                    .clone()
                    .unwrap_or_else(|| Key::Text(variant.ident.to_string())),
            }
        })
        .collect())
}

fn validate(attrs: &CborAttrs) -> darling::Result<()> {
    let profile = attrs.profile.unwrap_or(Profile::Rfc);

    if attrs.map.is_present() && attrs.array.is_present() {
        return Err(darling::Error::custom(
            "map and array are mutually exclusive",
        ));
    }

    match &attrs.data {
        Data::Struct(fields) => {
            match fields.style {
                Style::Unit => {
                    if attrs.map.is_present() {
                        return Err(darling::Error::custom("map is not valid on a unit struct"));
                    }
                    if attrs.array.is_present() {
                        return Err(darling::Error::custom(
                            "array is not valid on a unit struct",
                        ));
                    }
                }
                Style::Tuple if fields.fields.len() == 1 => {
                    if attrs.map.is_present() {
                        return Err(darling::Error::custom(
                            "map is not valid on a newtype struct",
                        ));
                    }
                    if attrs.array.is_present() {
                        return Err(darling::Error::custom(
                            "array is not valid on a newtype struct",
                        ));
                    }
                }
                Style::Tuple => {
                    if attrs.map.is_present() {
                        return Err(darling::Error::custom("map is not valid on a tuple struct"));
                    }
                }
                Style::Struct => {}
            }
            let (form, _) = struct_form(attrs)?;
            let fields = struct_fields(attrs)?;
            validate_fields(&fields, form, profile)?;
        }
        Data::Enum(variants) => {
            if attrs.map.is_present() || attrs.array.is_present() {
                return Err(darling::Error::custom(
                    "map and array are only valid on structs",
                ));
            }
            if variants.is_empty() {
                return Err(darling::Error::custom("an enum needs at least one variant"));
            }
            let variants = enum_variants(attrs)?;
            let mut keys = Vec::new();
            for variant in &variants {
                let _ = &variant.attrs.ident;
                if variant.attrs.tag.is_some() && variant.form == Form::Unit {
                    return Err(darling::Error::custom(
                        "a unit variant has no payload to tag",
                    ));
                }
                if keys.contains(&variant.key) {
                    return Err(darling::Error::custom("duplicate variant key"));
                }
                keys.push(variant.key.clone());
                validate_fields(&variant.fields, variant.form, profile)?;
            }
        }
    }

    Ok(())
}

fn validate_fields(fields: &[FieldInfo<'_>], form: Form, profile: Profile) -> darling::Result<()> {
    let mut keys = Vec::new();

    for field in fields {
        if field.attrs.key.is_some() && form != Form::Map {
            return Err(darling::Error::custom("key is only valid on map fields"));
        }

        if (field.attrs.default.is_some() || field.attrs.skip.is_present()) && form != Form::Map {
            return Err(darling::Error::custom(
                "default and skip are only valid on map fields",
            ));
        }

        if field.attrs.tag.is_some() && form == Form::Newtype {
            return Err(darling::Error::custom(
                "a newtype field cannot carry a tag; tag the newtype container",
            ));
        }

        if field.attrs.simple_value.is_present() && !is_u8(field.ty) {
            return Err(darling::Error::custom("simple_value requires a u8 field"));
        }

        if field.attrs.with.is_some() && field.attrs.simple_value.is_present() {
            return Err(darling::Error::custom(
                "with and simple_value are mutually exclusive",
            ));
        }

        if field.attrs.skip.is_present()
            && (field.attrs.key.is_some()
                || field.attrs.tag.is_some()
                || field.attrs.simple_value.is_present()
                || field.attrs.with.is_some()
                || field.attrs.indefinite.is_present())
        {
            return Err(darling::Error::custom(
                "skip cannot be combined with key, tag, simple_value, with, or indefinite",
            ));
        }

        if field.attrs.indefinite.is_present() {
            if field.attrs.with.is_some() || field.attrs.simple_value.is_present() {
                return Err(darling::Error::custom(
                    "indefinite cannot be combined with with or simple_value",
                ));
            }
            if !is_flat_collection(field.ty) {
                return Err(darling::Error::custom(
                    "indefinite requires a Vec<T>, BTreeSet<T>, or HashSet<T> field",
                ));
            }
            if profile != Profile::Rfc {
                return Err(darling::Error::custom(
                    "indefinite is forbidden under pref-plus and deterministic",
                ));
            }
        }

        match &field.attrs.default {
            Some(DefaultValue::Expr(expr)) => {
                let _ = expr;
            }
            Some(DefaultValue::Flag) | None => {}
        }

        if form == Form::Map && !field.attrs.skip.is_present() {
            let key = field.key();
            if keys.contains(&key) {
                return Err(darling::Error::custom("duplicate field key"));
            }
            keys.push(key);
        }
    }

    Ok(())
}

fn is_u8(ty: &Type) -> bool {
    path_ends_with(ty, "u8")
}

fn is_flat_collection(ty: &Type) -> bool {
    path_ends_with(ty, "Vec") || path_ends_with(ty, "BTreeSet") || path_ends_with(ty, "HashSet")
}

fn path_ends_with(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name),
        _ => false,
    }
}

struct UsedParams<'a> {
    declared: &'a [syn::Ident],
    used: HashSet<String>,
}

impl<'ast> Visit<'ast> for UsedParams<'_> {
    fn visit_type(&mut self, ty: &'ast Type) {
        if let Type::Path(path) = ty
            && path.qself.is_none()
            && let Some(segment) = path.path.segments.first()
            && self
                .declared
                .iter()
                .any(|declared| declared == &segment.ident)
        {
            self.used.insert(segment.ident.to_string());
        }
        visit::visit_type(self, ty);
    }
}

fn used_type_params<'a>(
    attrs: &CborAttrs,
    fields: impl IntoIterator<Item = &'a FieldInfo<'a>>,
) -> Vec<syn::Ident> {
    let declared: Vec<_> = attrs
        .generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();
    let used = {
        let mut visitor = UsedParams {
            declared: &declared,
            used: HashSet::new(),
        };
        for field in fields {
            if field.attrs.skip.is_present() || field.attrs.with.is_some() {
                continue;
            }
            visitor.visit_type(field.ty);
        }
        visitor.used
    };
    declared
        .into_iter()
        .filter(|ident| used.contains(&ident.to_string()))
        .collect()
}

fn serialize_generics<'a>(
    attrs: &CborAttrs,
    fields: impl IntoIterator<Item = &'a FieldInfo<'a>>,
) -> syn::Generics {
    let mut generics = attrs.generics.clone();
    for ident in used_type_params(attrs, fields) {
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#ident: ::seabored::ser::CborSerialize));
    }
    generics
}

fn unique_lifetime(generics: &syn::Generics) -> Lifetime {
    let mut name = String::from("de");
    while generics
        .lifetimes()
        .any(|param| param.lifetime.ident == name.as_str())
    {
        name.insert_str(0, "seabored_");
    }
    Lifetime::new(&format!("'{name}"), Span::call_site())
}

fn deserialize_generics<'a>(
    attrs: &CborAttrs,
    fields: impl IntoIterator<Item = &'a FieldInfo<'a>>,
) -> (syn::Generics, Lifetime) {
    let mut impl_generics = attrs.generics.clone();
    let de = unique_lifetime(&attrs.generics);
    impl_generics
        .params
        .insert(0, GenericParam::Lifetime(LifetimeParam::new(de.clone())));

    for param in attrs.generics.lifetimes() {
        let lifetime = param.lifetime.clone();
        impl_generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#de: #lifetime));
    }
    for ident in used_type_params(attrs, fields) {
        impl_generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#ident: ::seabored::de::CborDeserialize<#de> + #de));
    }

    (impl_generics, de)
}

fn generate(attrs: &CborAttrs, direction: Direction) -> syn::Result<TokenStream2> {
    match &attrs.data {
        Data::Struct(_) => generate_struct(attrs, direction),
        Data::Enum(_) => generate_enum(attrs, direction),
    }
}

fn generate_struct(attrs: &CborAttrs, direction: Direction) -> syn::Result<TokenStream2> {
    let (form, named) = struct_form(attrs)?;
    let fields = struct_fields(attrs)?;
    let profile = attrs.profile.unwrap_or(Profile::Rfc);

    match direction {
        Direction::Serialize => {
            let ident = &attrs.ident;
            let generics = serialize_generics(attrs, &fields);
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let body = encode_body(attrs, form, &fields, profile);
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics ::seabored::ser::CborSerialize for #ident #ty_generics #where_clause {
                    fn cbor_serialize_to<W: ::seabored::reexports::parsio::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<usize, ::seabored::error::SeaboredSerError> {
                        #body
                    }
                }
            })
        }
        Direction::Deserialize => {
            let ident = &attrs.ident;
            let (impl_generics, de) = deserialize_generics(attrs, &fields);
            let (impl_generics, _, where_clause) = impl_generics.split_for_impl();
            let (_, ty_generics, _) = attrs.generics.split_for_impl();
            let body = if let Some(tag) = attrs.tag {
                let check = tag_check(quote!(&mut *reader), tag, &de, profile);
                let inner = decode_body(
                    form,
                    named,
                    &fields,
                    quote!(&mut tagged_reader),
                    &de,
                    quote!(Self),
                    profile,
                );
                quote!({
                    #check
                    ::core::result::Result::Ok(#inner)
                })
            } else {
                let inner = decode_body(
                    form,
                    named,
                    &fields,
                    quote!(&mut *reader),
                    &de,
                    quote!(Self),
                    profile,
                );
                quote!(::core::result::Result::Ok(#inner))
            };
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics ::seabored::de::CborDeserialize<#de> for #ident #ty_generics #where_clause {
                    fn cbor_deserialize_from<R: ::seabored::reexports::parsio::Read<#de>>(
                        reader: &mut ::seabored::reexports::parsio::DepthAwareReaderGuard<'_, #de, R>,
                    ) -> ::core::result::Result<Self, ::seabored::error::SeaboredDeError>
                    where
                        Self: Sized + #de,
                    {
                        #body
                    }
                }
            })
        }
    }
}

fn generate_enum(attrs: &CborAttrs, direction: Direction) -> syn::Result<TokenStream2> {
    let variants = enum_variants(attrs)?;
    let ident = &attrs.ident;
    let profile = attrs.profile.unwrap_or(Profile::Rfc);

    match direction {
        Direction::Serialize => {
            let generics = serialize_generics(
                attrs,
                variants.iter().flat_map(|variant| variant.fields.iter()),
            );
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let body = enum_encode_body(attrs, &variants, profile);
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics ::seabored::ser::CborSerialize for #ident #ty_generics #where_clause {
                    fn cbor_serialize_to<W: ::seabored::reexports::parsio::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<usize, ::seabored::error::SeaboredSerError> {
                        #body
                    }
                }
            })
        }
        Direction::Deserialize => {
            let (impl_generics, de) = deserialize_generics(
                attrs,
                variants.iter().flat_map(|variant| variant.fields.iter()),
            );
            let (impl_generics, _, where_clause) = impl_generics.split_for_impl();
            let (_, ty_generics, _) = attrs.generics.split_for_impl();
            let body = if let Some(tag) = attrs.tag {
                let check = tag_check(quote!(&mut *reader), tag, &de, profile);
                let inner = enum_decode_body(&variants, quote!(&mut tagged_reader), &de, profile);
                quote!({
                    #check
                    #inner
                })
            } else {
                enum_decode_body(&variants, quote!(&mut *reader), &de, profile)
            };
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics ::seabored::de::CborDeserialize<#de> for #ident #ty_generics #where_clause {
                    fn cbor_deserialize_from<R: ::seabored::reexports::parsio::Read<#de>>(
                        reader: &mut ::seabored::reexports::parsio::DepthAwareReaderGuard<'_, #de, R>,
                    ) -> ::core::result::Result<Self, ::seabored::error::SeaboredDeError>
                    where
                        Self: Sized + #de,
                    {
                        #body
                    }
                }
            })
        }
    }
}

fn enum_encode_body(
    attrs: &CborAttrs,
    variants: &[VariantInfo<'_>],
    profile: Profile,
) -> TokenStream2 {
    let mut branches = Vec::new();
    for variant in variants {
        let (pattern, slots) = variant_pattern(variant);
        let branch = variant_encode_branch(variant, &slots, profile);
        branches.push(quote! {
            #pattern => {
                #branch
            }
        });
    }

    let mut prefix = Vec::new();
    if let Some(tag) = attrs.tag {
        prefix.push(quote! {
            written += ::seabored::ser::write_preamble(
                ::seabored::mt::MajorType::Tagged,
                #tag,
                writer,
            )?;
        });
    }

    quote! {
        let mut written = 0usize;
        #(#prefix)*
        match self {
            #(#branches),*
        }
        ::core::result::Result::Ok(written)
    }
}

fn variant_pattern(variant: &VariantInfo<'_>) -> (TokenStream2, Vec<syn::Ident>) {
    let ident = &variant.attrs.ident;
    let slots: Vec<_> = variant.fields.iter().map(FieldInfo::slot).collect();
    let pattern = match variant.form {
        Form::Unit => quote!(Self::#ident),
        Form::Newtype | Form::Array => quote!(Self::#ident(#(#slots),*)),
        Form::Map => {
            let fields: Vec<_> = variant
                .fields
                .iter()
                .map(|field| field.attrs.ident.as_ref().expect("named variant field"))
                .collect();
            quote!(Self::#ident { #(#fields: #slots),* })
        }
    };
    (pattern, slots)
}

fn variant_encode_branch(
    variant: &VariantInfo<'_>,
    slots: &[syn::Ident],
    profile: Profile,
) -> TokenStream2 {
    let key = encode_key(&variant.key);
    let tag = if let Some(tag) = variant.attrs.tag {
        quote! {
            written += ::seabored::ser::write_preamble(
                ::seabored::mt::MajorType::Tagged,
                #tag,
                writer,
            )?;
        }
    } else {
        quote!()
    };
    match variant.form {
        Form::Unit => quote! {
            written += #key;
        },
        Form::Newtype => {
            let slot = &slots[0];
            let value = encode_value_expr(&variant.fields[0], quote!(#slot));
            quote! {
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Map,
                    1,
                    writer,
                )?;
                written += #key;
                #tag
                written += #value;
            }
        }
        Form::Array => {
            let count = variant.fields.len();
            let mut steps = Vec::new();
            for (field, slot) in variant.fields.iter().zip(slots) {
                let value = encode_value_expr(field, quote!(#slot));
                steps.push(quote! {
                    written += #value;
                });
            }
            quote! {
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Map,
                    1,
                    writer,
                )?;
                written += #key;
                #tag
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Array,
                    #count as u64,
                    writer,
                )?;
                #(#steps)*
            }
        }
        Form::Map => {
            let mut entries: Vec<_> = variant
                .fields
                .iter()
                .zip(slots)
                .filter(|(field, _)| !field.attrs.skip.is_present())
                .collect();
            if profile == Profile::Deterministic {
                entries.sort_by(|(left, _), (right, _)| {
                    key_bytes(&left.key()).cmp(&key_bytes(&right.key()))
                });
            }
            let count = entries.len();
            let mut steps = Vec::new();
            for (field, slot) in entries {
                let field_key = encode_key(&field.key());
                let value = encode_value_expr(field, quote!(#slot));
                steps.push(quote! {
                    written += #field_key;
                    written += #value;
                });
            }
            quote! {
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Map,
                    1,
                    writer,
                )?;
                written += #key;
                #tag
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Map,
                    #count as u64,
                    writer,
                )?;
                #(#steps)*
            }
        }
    }
}

fn enum_decode_body(
    variants: &[VariantInfo<'_>],
    reader: TokenStream2,
    de: &Lifetime,
    profile: Profile,
) -> TokenStream2 {
    let strict = profile != Profile::Rfc;
    let deterministic = profile == Profile::Deterministic;
    let mut unit_text_arms = Vec::new();
    let mut unit_int_conditions = Vec::new();
    let mut payload_text_arms = Vec::new();
    let mut payload_int_conditions = Vec::new();

    let unit_map_error = quote! {
        return ::core::result::Result::Err(
            ::seabored::error::SeaboredDeError::IncorrectMajorType {
                actual: ::seabored::mt::MajorType::Map,
                expected: &[
                    ::seabored::mt::MajorType::String,
                    ::seabored::mt::MajorType::Uint,
                    ::seabored::mt::MajorType::NegativeUint,
                ],
            },
        );
    };

    for variant in variants {
        let ident = &variant.attrs.ident;
        let base_unit_action = quote! {
            return ::core::result::Result::Ok(Self::#ident);
        };
        let base_payload_action = if variant.form == Form::Unit {
            unit_map_error.clone()
        } else {
            payload_variant_arm(variant, de, profile)
        };

        match &variant.key {
            Key::Text(text) => {
                let text = LitStr::new(text, Span::call_site());
                let key_check = quote! {
                    if #strict
                        && (key_ai.is_indefinite()
                            || !key_ai.is_shortest(key.len() as u64))
                    {
                        return ::core::result::Result::Err(
                            ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                                what: "variant key",
                            },
                        );
                    }
                };
                if variant.form == Form::Unit {
                    unit_text_arms.push(quote! {
                        #text => {
                            #key_check
                            #base_unit_action
                        }
                    });
                }
                payload_text_arms.push(quote! {
                    #text => {
                        #key_check
                        #base_payload_action
                    }
                });
            }
            Key::Int(value) => {
                let literal = int_literal(*value);
                let key_check = quote! {
                    if #strict && !key_ai.is_shortest(key.argument()) {
                        return ::core::result::Result::Err(
                            ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                                what: "variant key",
                            },
                        );
                    }
                };
                if variant.form == Form::Unit {
                    unit_int_conditions.push((
                        literal.clone(),
                        quote! {
                            #key_check
                            #base_unit_action
                        },
                    ));
                }
                payload_int_conditions.push((
                    literal,
                    quote! {
                        #key_check
                        #base_payload_action
                    },
                ));
            }
        }
    }

    let unit_text = {
        let arms = if unit_text_arms.is_empty() {
            quote! {}
        } else {
            quote! { #(#unit_text_arms)* }
        };
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(#reader)?.ai();
            let key = <::std::borrow::Cow<#de, str> as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?;
            match key.as_ref() {
                #arms
                _ => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::UnexpectedKey {
                            key: key.into_owned(),
                        },
                    );
                }
            }
        }
    };

    let unit_int = {
        let fallback = quote! {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::UnexpectedKey {
                    key: ::std::format!("{}", key),
                },
            );
        };
        let chain = int_chain(unit_int_conditions, fallback);
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(#reader)?.ai();
            let key = <::seabored::types::CborInteger as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?;
            #chain
        }
    };

    let payload_text = {
        let arms = if payload_text_arms.is_empty() {
            quote! {}
        } else {
            quote! { #(#payload_text_arms)* }
        };
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.ai();
            let key = <::std::borrow::Cow<#de, str> as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
            match key.as_ref() {
                #arms
                _ => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::UnexpectedKey {
                            key: key.into_owned(),
                        },
                    );
                }
            }
        }
    };

    let payload_int = {
        let fallback = quote! {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::UnexpectedKey {
                    key: ::std::format!("{}", key),
                },
            );
        };
        let chain = int_chain(payload_int_conditions, fallback);
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.ai();
            let key = <::seabored::types::CborInteger as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
            #chain
        }
    };

    let payload_map = quote! {
        let ib = <::seabored::ib::InitialByte as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != ::seabored::mt::MajorType::Map {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[::seabored::mt::MajorType::Map],
                },
            );
        }
        let len = ai.length(#reader)?;
        if #strict {
            match len {
                ::core::option::Option::Some(len) if !ai.is_shortest(len) => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                            what: "map header",
                        },
                    );
                }
                ::core::option::Option::None if #deterministic => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                            what: "map header",
                        },
                    );
                }
                _ => {}
            }
        }
        if let ::core::option::Option::Some(len) = len {
            let got: usize = len.try_into()?;
            if got != 1 {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::WrongLen { expected: 1, got },
                );
            }
        }
        let mut map_reader = (#reader).enter()?;
        if len.is_none()
            && <::seabored::ib::InitialByte>::peek(&mut map_reader)?.raw()
                == ::seabored::ib::consts::IB_BREAK
        {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::WrongLen {
                    expected: 1,
                    got: 0,
                },
            );
        }

        let key_mt = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.mt();
        match key_mt {
            ::seabored::mt::MajorType::String => {
                #payload_text
            }
            ::seabored::mt::MajorType::Uint | ::seabored::mt::MajorType::NegativeUint => {
                #payload_int
            }
            _ => {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::IncorrectMajorType {
                        actual: key_mt,
                        expected: &[
                            ::seabored::mt::MajorType::String,
                            ::seabored::mt::MajorType::Uint,
                            ::seabored::mt::MajorType::NegativeUint,
                        ],
                    },
                );
            }
        }
    };

    quote!({
        let first = <::seabored::ib::InitialByte>::peek(#reader)?;
        let mt = first.mt();
        match mt {
            ::seabored::mt::MajorType::Map => {
                #payload_map
            }
            ::seabored::mt::MajorType::String => {
                #unit_text
            }
            ::seabored::mt::MajorType::Uint | ::seabored::mt::MajorType::NegativeUint => {
                #unit_int
            }
            _ => {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::IncorrectMajorType {
                        actual: mt,
                        expected: &[
                            ::seabored::mt::MajorType::Map,
                            ::seabored::mt::MajorType::String,
                            ::seabored::mt::MajorType::Uint,
                            ::seabored::mt::MajorType::NegativeUint,
                        ],
                    },
                );
            }
        }
    })
}

fn payload_variant_arm(variant: &VariantInfo<'_>, de: &Lifetime, profile: Profile) -> TokenStream2 {
    let ident = &variant.attrs.ident;
    let (tag_stmts, payload_reader) = if let Some(tag) = variant.attrs.tag {
        (
            tag_check(quote!(&mut map_reader), tag, de, profile),
            quote!(&mut tagged_reader),
        )
    } else {
        (quote!(), quote!(&mut map_reader))
    };

    let value = match variant.form {
        Form::Unit => unreachable!("unit variant has no payload arm"),
        Form::Newtype => {
            let value = decode_value(&variant.fields[0], payload_reader, de, profile);
            quote!(Self::#ident(#value))
        }
        Form::Array => decode_array(
            variant.named,
            &variant.fields,
            payload_reader,
            de,
            quote!(Self::#ident),
            profile,
        ),
        Form::Map => decode_map(
            &variant.fields,
            payload_reader,
            de,
            quote!(Self::#ident),
            profile,
        ),
    };

    quote! {{
        let value = {
            #tag_stmts
            #value
        };
        if len.is_none() {
            if <::seabored::ib::InitialByte>::peek(&mut map_reader)?.raw()
                == ::seabored::ib::consts::IB_BREAK
            {
                ::seabored::reexports::parsio::Read::advance(&mut map_reader, 1)?;
            } else {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::TooManyElements { expected: 1 },
                );
            }
        }
        return ::core::result::Result::Ok(value);
    }}
}

fn int_chain(conditions: Vec<(Literal, TokenStream2)>, fallback: TokenStream2) -> TokenStream2 {
    let mut chain = fallback;
    for (literal, action) in conditions.into_iter().rev() {
        chain = quote! {
            if key == ::seabored::types::CborInteger::from(#literal) {
                #action
            } else {
                #chain
            }
        };
    }
    chain
}

fn encode_body(
    attrs: &CborAttrs,
    form: Form,
    fields: &[FieldInfo<'_>],
    profile: Profile,
) -> TokenStream2 {
    let mut steps = Vec::new();

    if let Some(tag) = attrs.tag {
        steps.push(quote! {
            written += ::seabored::ser::write_preamble(
                ::seabored::mt::MajorType::Tagged,
                #tag,
                writer,
            )?;
        });
    }

    match form {
        Form::Unit => steps.push(quote! {
            written += ::seabored::reexports::parsio::Write::write(
                writer,
                &[::seabored::ib::consts::IB_NULL],
            )?;
        }),
        Form::Newtype => {
            let value = encode_value(&fields[0]);
            steps.push(quote! {
                written += #value;
            });
        }
        Form::Array => {
            let count = fields.len();
            steps.push(quote! {
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Array,
                    #count as u64,
                    writer,
                )?;
            });
            for field in fields {
                let value = encode_value(field);
                steps.push(quote! {
                    written += #value;
                });
            }
        }
        Form::Map => {
            let entries: Vec<_> = fields
                .iter()
                .filter(|field| !field.attrs.skip.is_present())
                .collect();
            let entries = if profile == Profile::Deterministic {
                sorted_fields(entries)
            } else {
                entries
            };
            let count = entries.len();
            steps.push(quote! {
                written += ::seabored::ser::write_preamble(
                    ::seabored::mt::MajorType::Map,
                    #count as u64,
                    writer,
                )?;
            });
            for field in entries {
                let key = encode_key(&field.key());
                let value = encode_value(field);
                steps.push(quote! {
                    written += #key;
                    written += #value;
                });
            }
        }
    }

    quote! {
        let mut written = 0usize;
        #(#steps)*
        ::core::result::Result::Ok(written)
    }
}

fn encode_key(key: &Key) -> TokenStream2 {
    match key {
        Key::Text(text) => {
            let text = LitStr::new(text, Span::call_site());
            quote! {
                <&str as ::seabored::ser::CborSerialize>::cbor_serialize_to(&#text, writer)?
            }
        }
        Key::Int(value) => {
            let literal = int_literal(*value);
            quote! {
                <::seabored::types::CborInteger as ::seabored::ser::CborSerialize>::cbor_serialize_to(
                    &::seabored::types::CborInteger::from(#literal),
                    writer,
                )?
            }
        }
    }
}

fn encode_value(field: &FieldInfo<'_>) -> TokenStream2 {
    let member = &field.member;
    encode_value_expr(field, quote!(&self.#member))
}

fn encode_value_expr(field: &FieldInfo<'_>, expr: TokenStream2) -> TokenStream2 {
    let inner = if let Some(path) = &field.attrs.with {
        quote!(#path::cbor_serialize_to(#expr, writer)?)
    } else if field.attrs.simple_value.is_present() {
        quote! {
            <::seabored::types::CborSimpleValue as ::seabored::ser::CborSerialize>::cbor_serialize_to(
                &::seabored::types::CborSimpleValue(*#expr),
                writer,
            )?
        }
    } else if field.attrs.indefinite.is_present() {
        quote! {{
            let mut n = ::seabored::reexports::parsio::Write::write(
                writer,
                &[::seabored::ib::consts::IB_ARRAY_SEQUENCE_START],
            )?;
            for item in #expr {
                n += ::seabored::ser::CborSerialize::cbor_serialize_to(item, writer)?;
            }
            n += ::seabored::reexports::parsio::Write::write(
                writer,
                &[::seabored::ib::consts::IB_BREAK],
            )?;
            n
        }}
    } else {
        quote! {
            ::seabored::ser::CborSerialize::cbor_serialize_to(#expr, writer)?
        }
    };

    if let Some(tag) = field.attrs.tag {
        quote! {{
            let mut n = ::seabored::ser::write_preamble(
                ::seabored::mt::MajorType::Tagged,
                #tag,
                writer,
            )?;
            n += #inner;
            n
        }}
    } else {
        inner
    }
}

fn key_bytes(key: &Key) -> Vec<u8> {
    let mut bytes = Vec::new();
    match key {
        Key::Text(text) => {
            bytes.push(0x60);
            push_argument(&mut bytes, text.len() as u64);
            bytes.extend_from_slice(text.as_bytes());
        }
        Key::Int(value) => {
            if *value >= 0 {
                bytes.push(0x00);
                push_argument(&mut bytes, *value as u64);
            } else {
                bytes.push(0x20);
                push_argument(&mut bytes, (-1 - *value) as u64);
            }
        }
    }
    bytes
}

fn push_argument(bytes: &mut Vec<u8>, value: u64) {
    if value <= 23 {
        bytes.push(value as u8);
    } else if value <= u8::MAX as u64 {
        bytes.push(24);
        bytes.push(value as u8);
    } else if value <= u16::MAX as u64 {
        bytes.push(25);
        bytes.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= u32::MAX as u64 {
        bytes.push(26);
        bytes.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        bytes.push(27);
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn sorted_fields<'a>(
    fields: impl IntoIterator<Item = &'a FieldInfo<'a>>,
) -> Vec<&'a FieldInfo<'a>> {
    let mut fields: Vec<_> = fields.into_iter().collect();
    fields.sort_by_key(|left| key_bytes(&left.key()));
    fields
}

fn int_literal(value: i128) -> Literal {
    if value >= 0 {
        Literal::u64_suffixed(value as u64)
    } else {
        Literal::i64_suffixed(value as i64)
    }
}

fn tag_check(reader: TokenStream2, tag: u64, de: &Lifetime, profile: Profile) -> TokenStream2 {
    let tag_literal = Literal::u64_suffixed(tag);
    let strict = profile != Profile::Rfc;
    quote! {
        let tag_ib = <::seabored::ib::InitialByte as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?;
        let (tag_mt, tag_ai) = tag_ib.mt_ai();
        if tag_mt != ::seabored::mt::MajorType::Tagged {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::IncorrectMajorType {
                    actual: tag_mt,
                    expected: &[::seabored::mt::MajorType::Tagged],
                },
            );
        }
        let actual_tag = tag_ai.length(#reader)?.ok_or(
            ::seabored::error::SeaboredDeError::NotWellFormed("indefinite-length tag"),
        )?;
        if #strict && !tag_ai.is_shortest(actual_tag) {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::NonPreferredEncoding { what: "tag" },
            );
        }
        if actual_tag != #tag_literal {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::WrongTag {
                    actual: actual_tag,
                    expected: #tag_literal,
                },
            );
        }
        let mut tagged_reader = (#reader).enter()?;
    }
}

fn decode_header(
    reader: TokenStream2,
    expected: TokenStream2,
    de: &Lifetime,
    profile: Profile,
    what: &'static str,
) -> TokenStream2 {
    let strict = profile != Profile::Rfc;
    let deterministic = profile == Profile::Deterministic;
    quote! {
        let ib = <::seabored::ib::InitialByte as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != #expected {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[#expected],
                },
            );
        }
        let len = ai.length(#reader)?;
        if #strict {
            match len {
                ::core::option::Option::Some(len) if !ai.is_shortest(len) => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding { what: #what },
                    );
                }
                ::core::option::Option::None if #deterministic => {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding { what: #what },
                    );
                }
                _ => {}
            }
        }
    }
}

fn decode_body(
    form: Form,
    named: bool,
    fields: &[FieldInfo<'_>],
    reader: TokenStream2,
    de: &Lifetime,
    target: TokenStream2,
    profile: Profile,
) -> TokenStream2 {
    match form {
        Form::Unit => quote!({
            let raw = ::seabored::reexports::parsio::Read::read_byte(#reader)?;
            if raw != ::seabored::ib::consts::IB_NULL {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::IncorrectInitialByte {
                        actual: raw,
                        expected: ::seabored::ib::consts::IB_NULL,
                    },
                );
            }
            #target
        }),
        Form::Newtype => {
            let value = decode_value(&fields[0], reader, de, profile);
            quote!(#target(#value))
        }
        Form::Array => decode_array(named, fields, reader, de, target, profile),
        Form::Map => decode_map(fields, reader, de, target, profile),
    }
}

fn decode_array(
    named: bool,
    fields: &[FieldInfo<'_>],
    reader: TokenStream2,
    de: &Lifetime,
    target: TokenStream2,
    profile: Profile,
) -> TokenStream2 {
    let count = fields.len();
    let header = decode_header(
        reader.clone(),
        quote!(::seabored::mt::MajorType::Array),
        de,
        profile,
        "array header",
    );

    let steps: Vec<_> = fields
        .iter()
        .map(|field| {
            let slot = field.slot();
            let value = decode_value(field, quote!(&mut array_reader), de, profile);
            quote! {
                let #slot = #value;
            }
        })
        .collect();

    let construct = construct(fields, Form::Array, named, target);

    quote!({
        #header
        match len {
            ::core::option::Option::Some(len) => {
                let len: usize = len.try_into()?;
                if len != #count {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::WrongLen {
                            expected: #count,
                            got: len,
                        },
                    );
                }
                let mut array_reader = (#reader).enter()?;
                #(#steps)*
                #construct
            }
            ::core::option::Option::None => {
                let mut array_reader = (#reader).enter()?;
                #(#steps)*
                if <::seabored::ib::InitialByte>::peek(&mut array_reader)?.raw()
                    == ::seabored::ib::consts::IB_BREAK
                {
                    ::seabored::reexports::parsio::Read::advance(&mut array_reader, 1)?;
                } else {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::TooManyElements { expected: #count },
                    );
                }
                #construct
            }
        }
    })
}

fn decode_map(
    fields: &[FieldInfo<'_>],
    reader: TokenStream2,
    de: &Lifetime,
    target: TokenStream2,
    profile: Profile,
) -> TokenStream2 {
    let strict = profile != Profile::Rfc;
    let deterministic = profile == Profile::Deterministic;
    let header = decode_header(
        reader.clone(),
        quote!(::seabored::mt::MajorType::Map),
        de,
        profile,
        "map header",
    );

    let ordered: Vec<&FieldInfo<'_>> = if deterministic {
        sorted_fields(fields.iter().filter(|field| !field.attrs.skip.is_present()))
    } else {
        Vec::new()
    };

    let mut declarations = Vec::new();
    for field in fields {
        let slot = field.slot();
        let ty = field.ty;
        if field.attrs.skip.is_present() {
            let default = field.default_expr();
            declarations.push(quote! {
                let #slot: #ty = #default;
            });
        } else {
            declarations.push(quote! {
                let mut #slot: ::core::option::Option<#ty> = ::core::option::Option::None;
            });
        }
    }

    let mut text_arms = Vec::new();
    let mut int_conditions = Vec::new();
    for field in fields {
        if field.attrs.skip.is_present() {
            continue;
        }
        let slot = field.slot();
        let name = &field.name;
        let value = decode_value(field, quote!(&mut map_reader), de, profile);
        let position = if deterministic {
            Some(
                ordered
                    .iter()
                    .position(|ordered| ordered.index == field.index)
                    .expect("field is in deterministic order"),
            )
        } else {
            None
        };

        let key_strict = match &field.key() {
            Key::Text(_) => quote! {
                if #strict
                    && (key_ai.is_indefinite()
                        || !key_ai.is_shortest(key.len() as u64))
                {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                            what: "map key",
                        },
                    );
                }
            },
            Key::Int(_) => quote! {
                if #strict && !key_ai.is_shortest(key.argument()) {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                            what: "map key",
                        },
                    );
                }
            },
        };

        let order = if let Some(position) = position {
            quote! {
                if expected != #position {
                    return ::core::result::Result::Err(
                        ::seabored::error::SeaboredDeError::NonPreferredEncoding {
                            what: "map key order",
                        },
                    );
                }
            }
        } else {
            quote!()
        };

        let advance = if deterministic {
            quote! {
                expected += 1;
            }
        } else {
            quote!()
        };

        let assign = quote! {
            if #slot.is_some() {
                return ::core::result::Result::Err(
                    ::seabored::error::SeaboredDeError::DuplicateField { name: #name },
                );
            }
            #key_strict
            #order
            #slot = ::core::option::Option::Some(#value);
            #advance
        };

        match field.key() {
            Key::Text(text) => {
                let text = LitStr::new(&text, Span::call_site());
                text_arms.push(quote! {
                    #text => { #assign }
                });
            }
            Key::Int(value) => {
                let literal = int_literal(value);
                int_conditions.push((literal, assign));
            }
        }
    }

    let unknown_text = if deterministic {
        quote! {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::UnexpectedKey {
                    key: key.into_owned(),
                },
            );
        }
    } else {
        quote! {
            ::seabored::de::skip_value(&mut map_reader)?;
        }
    };

    let unknown_int = if deterministic {
        quote! {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::UnexpectedKey {
                    key: ::std::format!("{}", key),
                },
            );
        }
    } else {
        quote! {
            ::seabored::de::skip_value(&mut map_reader)?;
        }
    };

    let text_branch = if text_arms.is_empty() {
        if deterministic {
            quote! {
                let key = <::std::borrow::Cow<#de, str> as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
                #unknown_text
            }
        } else {
            quote! {
                ::seabored::de::skip_value(&mut map_reader)?;
                ::seabored::de::skip_value(&mut map_reader)?;
            }
        }
    } else {
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.ai();
            let key = <::std::borrow::Cow<#de, str> as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
            match key.as_ref() {
                #(#text_arms)*
                _ => { #unknown_text }
            }
        }
    };

    let int_branch = if int_conditions.is_empty() {
        if deterministic {
            quote! {
                let key = <::seabored::types::CborInteger as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
                #unknown_int
            }
        } else {
            quote! {
                ::seabored::de::skip_value(&mut map_reader)?;
                ::seabored::de::skip_value(&mut map_reader)?;
            }
        }
    } else {
        let mut chain = unknown_int;
        for (literal, assign) in int_conditions.into_iter().rev() {
            chain = quote! {
                if key == ::seabored::types::CborInteger::from(#literal) {
                    #assign
                } else {
                    #chain
                }
            };
        }
        quote! {
            let key_ai = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.ai();
            let key = <::seabored::types::CborInteger as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(&mut map_reader)?;
            #chain
        }
    };

    let mut finish = Vec::new();
    for field in fields {
        let slot = field.slot();
        if field.attrs.skip.is_present() {
            continue;
        }
        let name = &field.name;
        if field.attrs.default.is_some() && !deterministic {
            let default = field.default_expr();
            finish.push(quote! {
                let #slot = #slot.unwrap_or_else(|| #default);
            });
        } else {
            finish.push(quote! {
                let #slot = #slot.ok_or(
                    ::seabored::error::SeaboredDeError::MissingField { name: #name },
                )?;
            });
        }
    }

    let construct = construct(fields, Form::Map, true, target);
    let expected_declaration =
        if deterministic && fields.iter().any(|field| !field.attrs.skip.is_present()) {
            quote! {
                let mut expected = 0usize;
            }
        } else {
            quote!()
        };
    let other_key = if deterministic {
        quote! {
            return ::core::result::Result::Err(
                ::seabored::error::SeaboredDeError::IncorrectMajorType {
                    actual: key_mt,
                    expected: &[
                        ::seabored::mt::MajorType::String,
                        ::seabored::mt::MajorType::Uint,
                        ::seabored::mt::MajorType::NegativeUint,
                    ],
                },
            );
        }
    } else {
        quote! {
            ::seabored::de::skip_value(&mut map_reader)?;
            ::seabored::de::skip_value(&mut map_reader)?;
        }
    };

    quote!({
        #header
        #(#declarations)*
        let mut remaining: ::core::option::Option<usize> = match len {
            ::core::option::Option::Some(len) => ::core::option::Option::Some(len.try_into()?),
            ::core::option::Option::None => ::core::option::Option::None,
        };
        let mut map_reader = (#reader).enter()?;
        #expected_declaration
        loop {
            if matches!(remaining, ::core::option::Option::Some(0)) {
                break;
            }
            if remaining.is_none()
                && <::seabored::ib::InitialByte>::peek(&mut map_reader)?.raw()
                    == ::seabored::ib::consts::IB_BREAK
            {
                ::seabored::reexports::parsio::Read::advance(&mut map_reader, 1)?;
                break;
            }

            let key_mt = <::seabored::ib::InitialByte>::peek(&mut map_reader)?.mt();
            match key_mt {
                ::seabored::mt::MajorType::String => {
                    #text_branch
                }
                ::seabored::mt::MajorType::Uint | ::seabored::mt::MajorType::NegativeUint => {
                    #int_branch
                }
                _ => {
                    #other_key
                }
            }

            if let ::core::option::Option::Some(remaining) = remaining.as_mut() {
                *remaining -= 1;
            }
        }
        #(#finish)*
        #construct
    })
}

fn decode_value(
    field: &FieldInfo<'_>,
    reader: TokenStream2,
    de: &Lifetime,
    profile: Profile,
) -> TokenStream2 {
    if let Some(tag) = field.attrs.tag {
        let check = tag_check(reader, tag, de, profile);
        let inner = decode_value_inner(field, quote!(&mut tagged_reader), de);
        quote!({{
            #check
            #inner
        }})
    } else {
        decode_value_inner(field, reader, de)
    }
}

fn decode_value_inner(field: &FieldInfo<'_>, reader: TokenStream2, de: &Lifetime) -> TokenStream2 {
    if let Some(path) = &field.attrs.with {
        quote!(#path::cbor_deserialize_from(#reader)?)
    } else if field.attrs.simple_value.is_present() {
        quote! {
            <::seabored::types::CborSimpleValue as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?.0
        }
    } else {
        let ty = field.ty;
        quote! {
            <#ty as ::seabored::de::CborDeserialize<#de>>::cbor_deserialize_from(#reader)?
        }
    }
}

fn construct(
    fields: &[FieldInfo<'_>],
    form: Form,
    named: bool,
    target: TokenStream2,
) -> TokenStream2 {
    let slots: Vec<_> = fields.iter().map(FieldInfo::slot).collect();
    match form {
        Form::Unit => target,
        Form::Newtype => quote!(#target(#(#slots),*)),
        Form::Array | Form::Map if named => {
            let idents: Vec<_> = fields
                .iter()
                .map(|field| field.attrs.ident.as_ref().expect("named field"))
                .collect();
            quote!(#target { #(#idents: #slots),* })
        }
        Form::Array => quote!(#target(#(#slots),*)),
        Form::Map => unreachable!("map fields are named"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromMeta;

    #[test]
    fn keys_parse_strings_integers_and_negatives() {
        let text: syn::Lit = syn::parse_quote!("name");
        assert_eq!(Key::from_value(&text).unwrap(), Key::Text("name".into()));

        assert_eq!(
            Key::from_expr(&syn::parse_quote!(42)).unwrap(),
            Key::Int(42)
        );
        assert_eq!(
            Key::from_expr(&syn::parse_quote!(-42)).unwrap(),
            Key::Int(-42)
        );
        assert!(Key::from_expr(&syn::parse_quote!(18446744073709551616)).is_err());
        assert!(Key::from_expr(&syn::parse_quote!(-9223372036854775809)).is_err());
    }

    #[test]
    fn profiles_parse_kebab_case() {
        assert_eq!(Profile::from_string("rfc").unwrap(), Profile::Rfc);
        assert_eq!(
            Profile::from_string("pref-plus").unwrap(),
            Profile::PrefPlus
        );
        assert_eq!(
            Profile::from_string("deterministic").unwrap(),
            Profile::Deterministic
        );
    }
}
