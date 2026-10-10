use super::N64;

pub const MEMPAK_SIZE: usize = 0x8000;
pub const MPK_PAGE_SIZE: usize = 256;
const CONTROLLER_COUNT: usize = 4;
const INODE_RESERVED_BYTES: usize = 10;
const INODE_CHECKSUM: u8 = 0x71;

const LABEL_AREA: [u8; 32] = [
    0x81, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

const MAIN_ID_AREA: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0x05, 0x1a, 0x5f, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0xff, 0x66, 0x25, 0x99, 0xcd,
];

const UNUSED_AREA: [u8; 32] = [0x00; 32];

const ID_AREA_BACKUP: [u8; 32] = MAIN_ID_AREA;

fn initialize_page_0() -> [u8; MPK_PAGE_SIZE] {
    let mut page = [0; MPK_PAGE_SIZE];

    for (slot, region) in [
        LABEL_AREA,
        MAIN_ID_AREA,
        UNUSED_AREA,
        ID_AREA_BACKUP,
        ID_AREA_BACKUP,
        UNUSED_AREA,
        ID_AREA_BACKUP,
        UNUSED_AREA,
    ]
    .iter()
    .enumerate()
    {
        let start = slot * region.len();
        page[start..start + region.len()].copy_from_slice(region);
    }

    page
}

fn format_mempak(bytes: &mut Vec<u8>) {
    if bytes.len() >= MEMPAK_SIZE * CONTROLLER_COUNT {
        return;
    }

    let existing_length = bytes.len();
    let first_new_bank = existing_length.div_ceil(MEMPAK_SIZE);
    bytes.resize(MEMPAK_SIZE * CONTROLLER_COUNT, 0);

    for bank in bytes.chunks_exact_mut(MEMPAK_SIZE).skip(first_new_bank) {
        bank[..MPK_PAGE_SIZE].copy_from_slice(&initialize_page_0());
        for inode in
            bank[MPK_PAGE_SIZE + INODE_RESERVED_BYTES..2 * MPK_PAGE_SIZE].chunks_exact_mut(2)
        {
            inode.copy_from_slice(&[0, 3]);
        }
        bank[MPK_PAGE_SIZE + 1] = INODE_CHECKSUM;
        bank.copy_within(MPK_PAGE_SIZE..2 * MPK_PAGE_SIZE, 2 * MPK_PAGE_SIZE);
        bank[3 * MPK_PAGE_SIZE..MEMPAK_SIZE - 3 * MPK_PAGE_SIZE].fill(0);
    }
}

fn handle_read_write(
    n64: &mut N64,
    channel: usize,
    address: u16,
    data: usize,
    size: usize,
    is_write: bool,
) {
    let buffers = n64.pak_buffers();
    let packet = &mut buffers.packet[data..data + size];

    if usize::from(address) >= MEMPAK_SIZE {
        packet.fill(0);
        return;
    }

    format_mempak(buffers.save);
    let offset = channel * MEMPAK_SIZE + usize::from(address);
    let saved = &mut buffers.save[offset..offset + size];

    if is_write {
        saved.copy_from_slice(packet);
        *buffers.dirty = true;
    } else {
        packet.copy_from_slice(saved);
    }
}

pub fn read(n64: &mut N64, channel: usize, address: u16, data: usize, size: usize) {
    handle_read_write(n64, channel, address, data, size, false);
}

pub fn write(n64: &mut N64, channel: usize, address: u16, data: usize, size: usize) {
    handle_read_write(n64, channel, address, data, size, true);
}
