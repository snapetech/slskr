use super::*;

#[derive(Debug)]
pub struct OverlayFramer<S> {
    stream: S,
}

impl<S> OverlayFramer<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    #[must_use]
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> OverlayFramer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn write<T: Serialize>(&mut self, message: &T) -> Result<(), OverlayError> {
        let payload = serde_json::to_vec(message)?;
        if payload.len() > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(payload.len()));
        }
        if payload.len() < 2 {
            return Err(OverlayError::FrameTooSmall(payload.len()));
        }
        let length =
            u32::try_from(payload.len()).map_err(|_| OverlayError::FrameTooLarge(payload.len()))?;
        self.stream.write_all(&length.to_be_bytes()).await?;
        self.stream.write_all(&payload).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn read_raw(&mut self) -> Result<Vec<u8>, OverlayError> {
        let mut header = [0_u8; 4];
        self.stream.read_exact(&mut header).await?;
        if header[0] == b'{' {
            return self.read_legacy_unframed(header).await;
        }
        let length = u32::from_be_bytes(header) as usize;
        if length < 2 {
            return Err(OverlayError::FrameTooSmall(length));
        }
        if length > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(length));
        }
        let mut payload = vec![0_u8; length];
        self.stream.read_exact(&mut payload).await?;
        Ok(payload)
    }

    pub async fn read<T: DeserializeOwned>(&mut self) -> Result<T, OverlayError> {
        Ok(serde_json::from_slice(&self.read_raw().await?)?)
    }

    async fn read_legacy_unframed(&mut self, header: [u8; 4]) -> Result<Vec<u8>, OverlayError> {
        let mut payload = header.to_vec();
        let mut depth = 0_i32;
        let mut in_string = false;
        let mut escaped = false;
        let mut complete = false;
        for byte in header {
            complete |=
                advance_json_object_boundary(byte, &mut depth, &mut in_string, &mut escaped);
        }
        loop {
            if complete {
                return match serde_json::from_slice::<Value>(&payload)? {
                    Value::Object(_) => Ok(payload),
                    _ => Err(OverlayError::InvalidJsonObject),
                };
            }
            if payload.len() >= MAX_OVERLAY_MESSAGE_BYTES {
                return Err(OverlayError::FrameTooLarge(payload.len()));
            }
            let byte = self.stream.read_u8().await?;
            payload.push(byte);
            complete = advance_json_object_boundary(byte, &mut depth, &mut in_string, &mut escaped);
        }
    }
}

fn advance_json_object_boundary(
    byte: u8,
    depth: &mut i32,
    in_string: &mut bool,
    escaped: &mut bool,
) -> bool {
    if *in_string {
        if *escaped {
            *escaped = false;
        } else if byte == b'\\' {
            *escaped = true;
        } else if byte == b'"' {
            *in_string = false;
        }
        return false;
    }

    match byte {
        b'"' => *in_string = true,
        b'{' => *depth += 1,
        b'}' => {
            *depth -= 1;
            return *depth == 0;
        }
        _ => {}
    }
    false
}
