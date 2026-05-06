use byteorder::{LittleEndian, ReadBytesExt};

const MAX_ITEMS: usize = 1024;

fn bounded_allocation<R: std::io::Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let raw = reader.read_u32::<LittleEndian>()?;
    let length = usize::try_from(raw).unwrap();
    if length > MAX_ITEMS {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "too many items"));
    }

    Ok(Vec::with_capacity(length))
}
