use proptest::prelude::*;
use slskr_protocol::{DecodeError, InitFrame, MessageFrame, RawFrame};

proptest! {
    #[test]
    fn message_frames_round_trip(code in any::<u32>(), payload in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let frame = MessageFrame::new(code, payload);
        let encoded = frame.encode()?;

        prop_assert_eq!(MessageFrame::decode(&encoded)?, frame);
    }

    #[test]
    fn init_frames_round_trip(code in any::<u8>(), payload in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let frame = InitFrame::new(code, payload);
        let encoded = frame.encode()?;

        prop_assert_eq!(InitFrame::decode(&encoded)?, frame);
    }

    #[test]
    fn raw_frames_round_trip(payload in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let frame = RawFrame::new(payload);

        prop_assert_eq!(RawFrame::decode(&frame.encode()), frame);
    }
}

#[test]
fn message_frame_layout_is_len_code_payload() {
    let frame = MessageFrame::new(5, [0xAA, 0xBB]);

    assert_eq!(
        frame.encode().unwrap(),
        [6, 0, 0, 0, 5, 0, 0, 0, 0xAA, 0xBB]
    );
}

#[test]
fn init_frame_layout_is_len_code_payload() {
    let frame = InitFrame::new(1, [0xAA, 0xBB]);

    assert_eq!(frame.encode().unwrap(), [3, 0, 0, 0, 1, 0xAA, 0xBB]);
}

#[test]
fn message_frame_rejects_len_shorter_than_code() {
    let input = [3, 0, 0, 0, 1, 2, 3];

    assert_eq!(
        MessageFrame::decode(&input),
        Err(DecodeError::InvalidFrameLength {
            length: 3,
            code_width: 4
        })
    );
}

#[test]
fn init_frame_rejects_zero_len() {
    let input = [0, 0, 0, 0];

    assert_eq!(
        InitFrame::decode(&input),
        Err(DecodeError::InvalidFrameLength {
            length: 0,
            code_width: 1
        })
    );
}
