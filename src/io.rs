pub trait ReadExt<'data>: parsio::Read<'data> {
    /// Provided implementation that hopefully erases the internal Cow for performance
    #[inline]
    fn read_be_u16(&mut self) -> parsio::ReadResult<u16> {
        self.read_array().map(|arr| u16::from_be_bytes(*arr))
    }

    /// Provided implementation that hopefully erases the internal Cow for performance
    #[inline]
    fn read_be_u32(&mut self) -> parsio::ReadResult<u32> {
        self.read_array().map(|arr| u32::from_be_bytes(*arr))
    }

    /// Provided implementation that hopefully erases the internal Cow for performance
    #[inline]
    fn read_be_u64(&mut self) -> parsio::ReadResult<u64> {
        self.read_array().map(|arr| u64::from_be_bytes(*arr))
    }
}

// Blanket impl
impl<'data, T> ReadExt<'data> for T where T: parsio::Read<'data> {}
