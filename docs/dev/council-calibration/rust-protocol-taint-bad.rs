use byteorder::{LittleEndian, ReadBytesExt};

fn unbounded_allocation<R: std::io::Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    Ok(Vec::with_capacity(length))
}

fn unbounded_loop<R: std::io::Read>(reader: &mut R) -> std::io::Result<usize> {
    let count = reader.read_u32::<LittleEndian>()? as usize;
    let mut total = 0;
    for _ in 0..count {
        total += 1;
    }

    Ok(total)
}
