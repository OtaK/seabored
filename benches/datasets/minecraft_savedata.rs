use crate::datasets::HasSample;

pub const BYTES: &[u8] = include_bytes!("cbor/minecraft_savedata.cbor");
pub const ID: &str = "minecraft-savedata";

#[derive(serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub count: i8,
    pub slot: u8,
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowItem<'a> {
    pub count: i8,
    pub slot: u8,
    pub id: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Abilities {
    pub walk_speed: f32,
    pub fly_speed: f32,
    pub may_fly: bool,
    pub flying: bool,
    pub invulnerable: bool,
    pub may_build: bool,
    pub instabuild: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub pos: (f64, f64, f64),
    pub motion: (f64, f64, f64),
    pub rotation: (f32, f32),
    pub fall_distance: f32,
    pub fire: u16,
    pub air: u16,
    pub on_ground: bool,
    pub no_gravity: bool,
    pub invulnerable: bool,
    pub portal_cooldown: i32,
    pub uuid: [u32; 4],
    pub custom_name: Option<String>,
    pub custom_name_visible: bool,
    pub silent: bool,
    pub glowing: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowEntity<'a> {
    pub id: &'a str,
    pub pos: (f64, f64, f64),
    pub motion: (f64, f64, f64),
    pub rotation: (f32, f32),
    pub fall_distance: f32,
    pub fire: u16,
    pub air: u16,
    pub on_ground: bool,
    pub no_gravity: bool,
    pub invulnerable: bool,
    pub portal_cooldown: i32,
    pub uuid: [u32; 4],
    pub custom_name: Option<&'a str>,
    pub custom_name_visible: bool,
    pub silent: bool,
    pub glowing: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RecipeBook {
    pub recipes: Vec<String>,
    pub to_be_displayed: Vec<String>,
    pub is_filtering_craftable: bool,
    pub is_gui_open: bool,
    pub is_furnace_filtering_craftable: bool,
    pub is_furnace_gui_open: bool,
    pub is_blasting_furnace_filtering_craftable: bool,
    pub is_blasting_furnace_gui_open: bool,
    pub is_smoker_filtering_craftable: bool,
    pub is_smoker_gui_open: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowRecipeBook<'a> {
    #[serde(borrow)]
    pub recipes: Vec<&'a str>,
    #[serde(borrow)]
    pub to_be_displayed: Vec<&'a str>,
    pub is_filtering_craftable: bool,
    pub is_gui_open: bool,
    pub is_furnace_filtering_craftable: bool,
    pub is_furnace_gui_open: bool,
    pub is_blasting_furnace_filtering_craftable: bool,
    pub is_blasting_furnace_gui_open: bool,
    pub is_smoker_filtering_craftable: bool,
    pub is_smoker_gui_open: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub game_type: GameType,
    pub previous_game_type: GameType,
    pub score: i64,
    pub dimension: String,
    pub selected_item_slot: u32,
    pub selected_item: Item,
    pub spawn_dimension: Option<String>,
    pub spawn_x: i64,
    pub spawn_y: i64,
    pub spawn_z: i64,
    pub spawn_forced: Option<bool>,
    pub sleep_timer: u16,
    pub food_exhaustion_level: f32,
    pub food_saturation_level: f32,
    pub food_tick_timer: u32,
    pub xp_level: u32,
    pub xp_p: f32,
    pub xp_total: i32,
    pub xp_seed: i32,
    pub inventory: Vec<Item>,
    pub ender_items: Vec<Item>,
    pub abilities: Abilities,
    pub entered_nether_position: Option<(f64, f64, f64)>,
    pub root_vehicle: Option<([u32; 4], Entity)>,
    pub shoulder_entity_left: Option<Entity>,
    pub shoulder_entity_right: Option<Entity>,
    pub seen_credits: bool,
    pub recipe_book: RecipeBook,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowPlayer<'a> {
    pub game_type: GameType,
    pub previous_game_type: GameType,
    pub score: i64,
    pub dimension: &'a str,
    pub selected_item_slot: u32,
    #[serde(borrow)]
    pub selected_item: BorrowItem<'a>,
    pub spawn_dimension: Option<&'a str>,
    pub spawn_x: i64,
    pub spawn_y: i64,
    pub spawn_z: i64,
    pub spawn_forced: Option<bool>,
    pub sleep_timer: u16,
    pub food_exhaustion_level: f32,
    pub food_saturation_level: f32,
    pub food_tick_timer: u32,
    pub xp_level: u32,
    pub xp_p: f32,
    pub xp_total: i32,
    pub xp_seed: i32,
    #[serde(borrow)]
    pub inventory: Vec<BorrowItem<'a>>,
    #[serde(borrow)]
    pub ender_items: Vec<BorrowItem<'a>>,
    pub abilities: Abilities,
    pub entered_nether_position: Option<(f64, f64, f64)>,
    #[serde(borrow)]
    pub root_vehicle: Option<([u32; 4], BorrowEntity<'a>)>,
    #[serde(borrow)]
    pub shoulder_entity_left: Option<BorrowEntity<'a>>,
    #[serde(borrow)]
    pub shoulder_entity_right: Option<BorrowEntity<'a>>,
    pub seen_credits: bool,
    #[serde(borrow)]
    pub recipe_book: BorrowRecipeBook<'a>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Players {
    pub players: Vec<Player>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowPlayers<'a> {
    #[serde(borrow)]
    pub players: Vec<BorrowPlayer<'a>>,
}

impl HasSample for Players {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}

impl HasSample for BorrowPlayers<'_> {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}
