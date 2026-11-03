use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite};

use word_guess_assistant_lib::codec::{
    game_message::Data, ChatMessage, GameMessage, GamePhase, IdentityAssignment, JoinRequest,
    ProtobufCodec, SyncPhase, VoteAction,
};

#[tokio::test]
async fn test_mock_bots_e2e_full_lifecycle() {
    // 1. 启动一个临时端口的 TCP 游戏服务端
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("绑定测试端口失败");
    let server_addr: SocketAddr = listener.local_addr().unwrap();
    let port = server_addr.port();

    println!(">>> 测试服务端已启动在端口: {}", port);

    // 2. 服务端协程：接收 4 名机器人连接并推进对局
    let server_task = tokio::spawn(async move {
        let mut client_writers = HashMap::new();
        let mut joined_bots = Vec::new();

        // 阶段 A: 等待 4 名特工接入并接收 JoinRequest
        for _ in 0..4 {
            let (stream, _addr) = listener.accept().await.expect("接收客户端连接失败");
            let (read_half, write_half) = stream.into_split();
            let mut reader = FramedRead::new(read_half, ProtobufCodec::new());
            let writer = FramedWrite::new(write_half, ProtobufCodec::new());

            let join_msg = reader.next().await.unwrap().unwrap();
            if let Some(Data::JoinReq(req)) = join_msg.data {
                println!("  [服务端] 收到入房请求: {} ({})", req.player_name, req.player_id);
                let pid = req.player_id.clone();
                joined_bots.push(pid.clone());
                client_writers.insert(pid, (reader, writer));
            }
        }

        assert_eq!(joined_bots.len(), 4, "应当有 4 名特工成功加入");

        // 阶段 B: 服务端分发身份 IdentityAssignment
        let roles = ["GOOD", "SPY", "ANGEL", "BLANK"];
        for (idx, pid) in joined_bots.iter().enumerate() {
            let (_, writer) = client_writers.get_mut(pid).unwrap();
            let id_msg = GameMessage {
                data: Some(Data::Identity(IdentityAssignment {
                    role: roles[idx].to_string(),
                    word: if roles[idx] == "BLANK" { String::new() } else { "苹果".to_string() },
                    hint: if roles[idx] == "BLANK" { "一种水果".to_string() } else { String::new() },
                })),
            };
            writer.send(id_msg).await.unwrap();
        }

        // 阶段 C: 推进至描述阶段 Description 并收集全员发言
        for pid in joined_bots.iter() {
            let (_, writer) = client_writers.get_mut(pid).unwrap();
            let phase_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::Description as i32,
                    extra_info: "第1轮一句话描述".to_string(),
                })),
            };
            writer.send(phase_msg).await.unwrap();
        }

        // 验证全员提交了描述
        for pid in joined_bots.iter() {
            let (reader, _) = client_writers.get_mut(pid).unwrap();
            let res = tokio::time::timeout(Duration::from_secs(5), reader.next()).await;
            assert!(res.is_ok(), "等待特工发言超时");
            let desc_msg = res.unwrap().unwrap().unwrap();
            if let Some(Data::Chat(chat)) = desc_msg.data {
                println!("  [服务端] 收到特工【{}】描述: {}", chat.sender_id, chat.text);
                assert_eq!(chat.sender_id, *pid);
                assert!(!chat.text.is_empty());
            } else {
                panic!("期望收到 ChatMessage 描述发言");
            }
        }

        // 阶段 D: 推进至投票阶段 Voting 并收集选票
        for pid in joined_bots.iter() {
            let (_, writer) = client_writers.get_mut(pid).unwrap();
            let phase_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::Voting as i32,
                    extra_info: "全员投票".to_string(),
                })),
            };
            writer.send(phase_msg).await.unwrap();
        }

        // 验证全员提交了公投票
        for pid in joined_bots.iter() {
            let (reader, _) = client_writers.get_mut(pid).unwrap();
            let res = tokio::time::timeout(Duration::from_secs(6), reader.next()).await;
            assert!(res.is_ok(), "等待特工投票超时");
            let vote_msg = res.unwrap().unwrap().unwrap();
            if let Some(Data::Vote(vote)) = vote_msg.data {
                println!("  [服务端] 收到特工【{}】投票 -> 目标【{}】", vote.voter_id, vote.target_id);
                assert_eq!(vote.voter_id, *pid);
                assert!(!vote.target_id.is_empty());
            } else {
                panic!("期望收到 VoteAction 投票行为");
            }
        }

        // 阶段 E: 下发终局结算战报
        for pid in joined_bots.iter() {
            let (_, writer) = client_writers.get_mut(pid).unwrap();
            let settlement_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::Settlement as i32,
                    extra_info: "对局结束，好人阵营获胜！".to_string(),
                })),
            };
            writer.send(settlement_msg).await.unwrap();
        }

        println!("  [服务端] 端到端对局全流程生命周期闭环验证成功！");
    });

    // 3. 客户端协程：启动 4 名真实 Mock Bots 运行逻辑
    let bots_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        use tokio::net::TcpStream;

        let bot_configs = [
            ("bot_alpha", "🤖 AI特工·阿法", "/avatar/cyber/hacker.svg", "我拿到的词汇感觉大家经常会接触！"),
            ("bot_beta", "🐼 AI特工·贝塔", "/avatar/animals/panda.svg", "我这边的词汇特征十分鲜明！"),
            ("bot_gamma", "🦊 AI特工·伽马", "/avatar/animals/fox.svg", "我的这个词不仅实用，而且辨识度高！"),
            ("bot_delta", "🐱 AI特工·德尔塔", "/avatar/animals/cat.svg", "词义清晰明确，请大家放心！"),
        ];

        let mut bot_handles = Vec::new();
        for (id, name, avatar, desc) in bot_configs {
            let handle = tokio::spawn(async move {
                let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
                let (read_half, write_half) = stream.into_split();
                let mut reader = FramedRead::new(read_half, ProtobufCodec::new());
                let mut writer = FramedWrite::new(write_half, ProtobufCodec::new());

                // 发送 JoinReq
                writer.send(GameMessage {
                    data: Some(Data::JoinReq(JoinRequest {
                        player_id: id.to_string(),
                        player_name: name.to_string(),
                        avatar: avatar.to_string(),
                    })),
                }).await.unwrap();

                // 状态机处理
                while let Some(Ok(msg)) = reader.next().await {
                    if let Some(data) = msg.data {
                        match data {
                            Data::Identity(_) => {}
                            Data::SyncPhase(phase) => {
                                if phase.new_phase == GamePhase::Description as i32 {
                                    tokio::time::sleep(Duration::from_millis(50)).await;
                                    writer.send(GameMessage {
                                        data: Some(Data::Chat(ChatMessage {
                                            sender_id: id.to_string(),
                                            text: desc.to_string(),
                                            is_system: false,
                                        })),
                                    }).await.unwrap();
                                } else if phase.new_phase == GamePhase::Voting as i32 {
                                    tokio::time::sleep(Duration::from_millis(80)).await;
                                    writer.send(GameMessage {
                                        data: Some(Data::Vote(VoteAction {
                                            voter_id: id.to_string(),
                                            target_id: "bot_beta".to_string(),
                                        })),
                                    }).await.unwrap();
                                } else if phase.new_phase == GamePhase::Settlement as i32 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });
            bot_handles.push(handle);
        }

        for h in bot_handles {
            h.await.unwrap();
        }
    });

    let (server_res, bots_res) = tokio::join!(server_task, bots_task);
    server_res.unwrap();
    bots_res.unwrap();
}
