pub(crate) struct SavePacketBuffers<'a> {
    pub save: &'a mut Vec<u8>,
    pub dirty: &'a mut bool,
    pub packet: &'a mut [u8; 64],
}
