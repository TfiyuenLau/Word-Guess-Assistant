use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use std::io;
use tokio_util::codec::{Decoder, Encoder};

/// Protobuf 自动生成的游戏网络协议代码模块
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/game.rs"));
}

pub use proto::*;

/// 限制单条网络数据包的最大大小为 16MB，防止异常数据包导致内存溢出
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// 基于 Protobuf 的帧编解码器 (4 字节小端序长度前缀 + Protobuf 二进制消息体)
///
/// 协议格式：
/// ```text
/// +---------------------------+-----------------------------------+
/// | 消息体长度 (4 字节, u32_le) | Protobuf 二进制序列化数据 (N 字节)    |
/// +---------------------------+-----------------------------------+
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProtobufCodec;

impl ProtobufCodec {
    /// 创建一个新的 ProtobufCodec 实例
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for ProtobufCodec {
    type Item = GameMessage;
    type Error = io::Error;

    /// 解码 TCP 数据流中的数据包，处理粘包与半包
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 1. 如果当前接收到的数据不足 4 字节，无法读取长度头，等待更多数据
        if src.len() < 4 {
            return Ok(None);
        }

        // 2. 读取 4 字节小端序长度字段（此时不移动读指针，避免半包丢弃数据）
        let length = u32::from_le_bytes([src[0], src[1], src[2], src[3]]) as usize;

        // 3. 校验消息长度是否超过允许的最大阈值
        if length > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Frame length {} exceeds maximum allowed frame size {}",
                    length, MAX_FRAME_SIZE
                ),
            ));
        }

        // 4. 检查是否接收完整包（4 字节长度头 + length 字节消息体）
        let total_length = 4 + length;
        if src.len() < total_length {
            // 半包情况：预留足够的空间以便后续网络 IO 快速读取，并返回 Ok(None)
            src.reserve(total_length - src.len());
            return Ok(None);
        }

        // 5. 数据完整：跳过 4 字节长度头
        src.advance(4);

        // 6. 截取完整消息体并反序列化
        let payload = src.split_to(length);
        match GameMessage::decode(payload) {
            Ok(msg) => Ok(Some(msg)),
            Err(err) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to decode GameMessage from protobuf payload: {}", err),
            )),
        }
    }
}

impl Encoder<GameMessage> for ProtobufCodec {
    type Error = io::Error;

    /// 将 GameMessage 编码为带 4 字节小端序长度头的二进制字节流
    fn encode(&mut self, item: GameMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg_len = item.encoded_len();
        if msg_len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Encoded message size {} exceeds maximum frame size {}",
                    msg_len, MAX_FRAME_SIZE
                ),
            ));
        }

        // 预留 4 字节长度头 + 消息体的缓冲区空间
        dst.reserve(4 + msg_len);

        // 写入 4 字节小端序长度
        dst.put_u32_le(msg_len as u32);

        // 序列化 Protobuf 消息体
        item.encode(dst).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to encode GameMessage into protobuf: {}", err),
            )
        })?;

        Ok(())
    }
}

impl Encoder<&GameMessage> for ProtobufCodec {
    type Error = io::Error;

    /// 支持引用形式编码，避免在广播发送时产生额外的所有权复制
    fn encode(&mut self, item: &GameMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg_len = item.encoded_len();
        if msg_len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Encoded message size {} exceeds maximum frame size {}",
                    msg_len, MAX_FRAME_SIZE
                ),
            ));
        }

        dst.reserve(4 + msg_len);
        dst.put_u32_le(msg_len as u32);
        item.encode(dst).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to encode GameMessage into protobuf: {}", err),
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::game_message::Data;

    #[test]
    fn test_encode_decode_round_trip() {
        let mut codec = ProtobufCodec::new();
        let mut buffer = BytesMut::new();

        let original_msg = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "player_1".to_string(),
                text: "你好，世界！".to_string(),
                is_system: false,
            })),
        };

        // 编码
        codec.encode(&original_msg, &mut buffer).expect("encode should succeed");

        // 长度至少为 4 字节头部
        assert!(buffer.len() > 4);

        // 解码
        let decoded_msg = codec.decode(&mut buffer).expect("decode should succeed");
        assert!(decoded_msg.is_some());
        assert_eq!(decoded_msg.unwrap(), original_msg);

        // 缓冲区应完全被消耗
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_partial_packet_handling() {
        let mut codec = ProtobufCodec::new();
        let mut buffer = BytesMut::new();

        let original_msg = GameMessage {
            data: Some(Data::JoinReq(JoinRequest {
                player_id: "host_01".to_string(),
                player_name: "房主".to_string(),
                avatar: "avatar_01.svg".to_string(),
            })),
        };

        // 编码到临时缓冲区
        let mut full_bytes = BytesMut::new();
        codec.encode(original_msg.clone(), &mut full_bytes).unwrap();

        // 1. 模拟网络只到了前 2 字节（不足 4 字节头）
        buffer.extend_from_slice(&full_bytes[0..2]);
        assert_eq!(codec.decode(&mut buffer).unwrap(), None);

        // 2. 模拟网络到了头部的剩余 2 字节以及消息体的一半
        buffer.extend_from_slice(&full_bytes[2..6]);
        assert_eq!(codec.decode(&mut buffer).unwrap(), None);

        // 3. 发送剩余的所有数据
        buffer.extend_from_slice(&full_bytes[6..]);
        let decoded = codec.decode(&mut buffer).unwrap();
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), original_msg);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_sticky_packets_handling() {
        let mut codec = ProtobufCodec::new();
        let mut buffer = BytesMut::new();

        let msg1 = GameMessage {
            data: Some(Data::Heartbeat(Heartbeat {
                player_id: "p1".to_string(),
                timestamp: 123456789,
            })),
        };

        let msg2 = GameMessage {
            data: Some(Data::SyncPhase(SyncPhase {
                new_phase: GamePhase::Description as i32,
                extra_info: "首轮发言".to_string(),
            })),
        };

        // 两个包粘在一起连续发送
        codec.encode(&msg1, &mut buffer).unwrap();
        codec.encode(&msg2, &mut buffer).unwrap();

        // 第一次解码出 msg1
        let res1 = codec.decode(&mut buffer).unwrap();
        assert_eq!(res1, Some(msg1));

        // 第二次解码出 msg2
        let res2 = codec.decode(&mut buffer).unwrap();
        assert_eq!(res2, Some(msg2));

        // 此时缓冲区清空，继续解码返回 None
        let res3 = codec.decode(&mut buffer).unwrap();
        assert_eq!(res3, None);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_oversized_frame_rejected() {
        let mut codec = ProtobufCodec::new();
        let mut buffer = BytesMut::new();

        // 伪造一个超过 MAX_FRAME_SIZE 的 4 字节长度头 (例如 20MB)
        let huge_len = (MAX_FRAME_SIZE + 1024) as u32;
        buffer.put_u32_le(huge_len);

        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
}

