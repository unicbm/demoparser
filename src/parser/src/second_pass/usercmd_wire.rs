//! Borrow usercmd payloads from the packet scratch buffer. Validate the entire
//! envelope before applying commands, matching prost's malformed-message behavior.
use prost::encoding::{decode_key, decode_varint, skip_field, DecodeContext, WireType};
use prost::{DecodeError, Message};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CommandRange {
    pub data: (usize, usize),
    pub delta: (usize, usize),
    pub player_slot: i32,
}

fn delimited<'a>(wire: WireType, bytes: &mut &'a [u8]) -> Result<&'a [u8], DecodeError> {
    if wire != WireType::LengthDelimited {
        return Err(DecodeError::new("invalid usercmd wire type"));
    }
    let length = decode_varint(bytes)?;
    if length > bytes.len() as u64 {
        return Err(DecodeError::new("truncated usercmd envelope"));
    }
    let (value, rest) = bytes.split_at(length as usize);
    *bytes = rest;
    Ok(value)
}

pub(crate) fn decode_commands(input: &[u8], commands: &mut Vec<CommandRange>) -> Result<(), DecodeError> {
    commands.clear();
    let mut bytes = input;
    while !bytes.is_empty() {
        let (tag, wire) = decode_key(&mut bytes)?;
        if tag != 1 {
            skip_field(wire, tag, &mut bytes, DecodeContext::default())?;
            continue;
        }
        let mut message = delimited(wire, &mut bytes)?;
        let mut command = CommandRange {
            data: (0, 0),
            delta: (0, 0),
            player_slot: -1,
        };
        while !message.is_empty() {
            let (tag, wire) = decode_key(&mut message)?;
            match tag {
                1 | 6 => {
                    let payload = delimited(wire, &mut message)?;
                    // Both slices originate in input; only integer offsets are
                    // retained so no borrow can outlive this packet.
                    let range = (payload.as_ptr() as usize - input.as_ptr() as usize, payload.len());
                    if tag == 1 {
                        command.data = range;
                    } else {
                        command.delta = range;
                    }
                }
                2..=5 => {
                    if wire != WireType::Varint {
                        return Err(DecodeError::new("invalid usercmd scalar wire type"));
                    }
                    let value = decode_varint(&mut message)? as i32;
                    if tag == 3 {
                        command.player_slot = value;
                    }
                }
                _ => {
                    // Prost counts the enclosing command against its group
                    // recursion limit. Use its validator for this rare case.
                    if wire == WireType::StartGroup {
                        csgoproto::CsvcMsgUserCommands::decode(input)?;
                    }
                    skip_field(wire, tag, &mut message, DecodeContext::default())?;
                }
            }
        }
        commands.push(command);
    }
    Ok(())
}

/// Validate full history entries with the generated protobuf decoder without
/// building the list. All other fields retain ordinary protobuf merge semantics.
pub(crate) fn merge_with_history_len(mut bytes: &[u8], command: &mut csgoproto::CsgoUserCmdPb) -> Result<usize, DecodeError> {
    let mut count = 0;
    while !bytes.is_empty() {
        let (tag, wire) = decode_key(&mut bytes)?;
        if tag == 2 {
            // merge_repeated uses this same merger/context before appending.
            prost::encoding::message::merge(wire, &mut csgoproto::CsgoInputHistoryEntryPb::default(),
                &mut bytes, DecodeContext::default())?;
            count += 1;
        } else {
            command.merge_field(tag, wire, &mut bytes, DecodeContext::default())?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use csgoproto::{CMsgServerUserCmd, CsvcMsgUserCommands};
    use prost::Message;

    #[test]
    fn history_length_projection_matches_full_protobuf_validation() {
        use csgoproto::{CBaseUserCmdPb, CMsgQAngle, CsgoInputHistoryEntryPb, CsgoUserCmdPb};
        fn check(bytes: &[u8]) {
            let expected = CsgoUserCmdPb::decode(bytes);
            let mut actual = CsgoUserCmdPb::default();
            let count = merge_with_history_len(bytes, &mut actual);
            assert_eq!(count.is_ok(), expected.is_ok(), "{bytes:x?}");
            if let Ok(mut expected) = expected {
                assert_eq!(count.unwrap(), expected.input_history.len());
                assert!(actual.input_history.is_empty());
                expected.input_history.clear();
                assert_eq!(actual.encode_to_vec(), expected.encode_to_vec());
            }
        }
        let mut bytes = CsgoUserCmdPb {
            input_history: (0..20).map(|tick| CsgoInputHistoryEntryPb {
                render_tick_count: Some(tick),
                view_angles: Some(CMsgQAngle { x: Some(-0.0), y: Some(2.5), z: None }),
                ..Default::default()
            }).collect(),
            base: Some(CBaseUserCmdPb { client_tick: Some(123), ..Default::default() }),
            ..Default::default()
        }.encode_to_vec();
        // Singular children merge across occurrences; histories append.
        bytes.extend(CsgoUserCmdPb {
            base: Some(CBaseUserCmdPb { pawn_entity_handle: Some(9), ..Default::default() }),
            input_history: vec![CsgoInputHistoryEntryPb::default()], ..Default::default()
        }.encode_to_vec());
        check(&bytes);
        for end in 0..bytes.len() { check(&bytes[..end]); }
        for index in 0..bytes.len() {
            for byte in [0, 7, 0x80, 0xff] {
                let mut corrupt = bytes.clone();
                corrupt[index] = byte;
                check(&corrupt);
            }
        }
        // Unknown nested groups must retain the generated decoder's depth limit.
        for depth in [98, 99, 100, 101] {
            let mut entry = vec![0x53; depth];
            entry.extend(std::iter::repeat_n(0x54, depth));
            let mut bytes = vec![0x12];
            prost::encoding::encode_varint(entry.len() as u64, &mut bytes);
            bytes.extend(entry);
            check(&bytes);
        }
    }

    fn check(bytes: &[u8], scratch: &mut Vec<CommandRange>) {
        let oracle = CsvcMsgUserCommands::decode(bytes);
        let actual = decode_commands(bytes, scratch);
        assert_eq!(actual.is_ok(), oracle.is_ok(), "wire: {bytes:?}");
        if let Ok(oracle) = oracle {
            assert_eq!(scratch.len(), oracle.commands.len());
            for (actual, expected) in scratch.iter().zip(oracle.commands) {
                assert_eq!(actual.player_slot, expected.player_slot());
                assert_eq!(&bytes[actual.data.0..actual.data.0 + actual.data.1], expected.data());
                assert_eq!(&bytes[actual.delta.0..actual.delta.0 + actual.delta.1], expected.delta_data());
            }
        }
    }

    #[test]
    fn borrowed_envelopes_match_prost_with_reused_storage_and_malformed_input() {
        let message = CsvcMsgUserCommands {
            commands: vec![
                CMsgServerUserCmd {
                    data: Some(vec![1, 2, 3].into()),
                    player_slot: Some(0),
                    ..Default::default()
                },
                CMsgServerUserCmd {
                    delta_data: Some(vec![4, 5].into()),
                    player_slot: Some(-7),
                    cmd_number: Some(-1),
                    ..Default::default()
                },
                CMsgServerUserCmd::default(),
            ],
        }
        .encode_to_vec();
        let mut scratch = Vec::new();
        check(&message, &mut scratch);
        for depth in [98, 99, 100, 101] {
            let mut nested = vec![59; depth];
            nested.extend(std::iter::repeat_n(60, depth));
            let mut bytes = vec![10];
            prost::encoding::encode_varint(nested.len() as u64, &mut bytes);
            bytes.extend(nested);
            check(&bytes, &mut scratch);
        }
        // Truncation or corruption of a later envelope must reject the whole
        // message before any earlier command can reach parser state.
        for end in 0..message.len() {
            check(&message[..end], &mut scratch);
        }
        for index in 0..message.len() {
            for byte in [0, 7, 0x80, 0xff] {
                let mut corrupt = message.clone();
                corrupt[index] = byte;
                check(&corrupt, &mut scratch);
            }
        }
        // Repeated singular bytes/slot fields use the last value, including an
        // explicit empty delta. Unknown fields and groups retain prost behavior.
        for bytes in [
            vec![10, 12, 10, 1, 1, 10, 1, 2, 24, 3, 24, 4, 50, 0],
            vec![10, 4, 59, 60, 24, 9, 67, 68],
            vec![10, 0],
            vec![],
            vec![10, 1, 0],
        ] {
            check(&bytes, &mut scratch);
        }
        check(&message, &mut scratch);
    }
}
