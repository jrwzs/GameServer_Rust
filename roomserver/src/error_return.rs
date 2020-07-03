use protobuf::Message;
use tools::cmd_code::ClientCode;
use tools::protos::protocol::{S_MODIFY_NICK_NAME, S_SYNC_DATA, S_USER_LOGIN};
use tools::protos::room::{
    S_CHANGE_TEAM, S_CHOOSE_CHARACTER, S_EMOJI, S_KICK_MEMBER, S_LEAVE_ROOM, S_PREPARE_CANCEL,
    S_ROOM, S_ROOM_SETTING, S_START,
};
use tools::tcp::TcpSender;
use tools::util::packet::Packet;

pub fn err_back(cmd: ClientCode, user_id: u32, error_mess: String, sender: &mut TcpSender) {
    match cmd {
        ClientCode::Login => {
            let mut sul = S_USER_LOGIN::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::SyncData => {
            let mut sul = S_SYNC_DATA::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::NickNameModify => {
            let mut sul = S_MODIFY_NICK_NAME::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::Room => {
            let mut sul = S_ROOM::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::LeaveRoom => {
            let mut sul = S_LEAVE_ROOM::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::Start => {
            let mut sul = S_START::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::ChangeTeam => {
            let mut sul = S_CHANGE_TEAM::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::PrepareCancel => {
            let mut sul = S_PREPARE_CANCEL::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::RoomSetting => {
            let mut sul = S_ROOM_SETTING::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::RoomMemberNotice => {}
        ClientCode::KickMember => {
            let mut sul = S_KICK_MEMBER::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::ChooseCharacter => {
            let mut sul = S_CHOOSE_CHARACTER::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::RoomNotice => {}
        ClientCode::Emoji => {
            let mut sul = S_EMOJI::new();
            sul.err_mess = error_mess;
            sul.is_succ = false;
            let bytes = Packet::build_packet_bytes(
                cmd as u32,
                user_id,
                sul.write_to_bytes().unwrap(),
                true,
                true,
            );
            sender.write(bytes);
        }
        ClientCode::EmojiNotice => {}
        ClientCode::MemberLeaveNotice => {}
        _ => {}
    }
}