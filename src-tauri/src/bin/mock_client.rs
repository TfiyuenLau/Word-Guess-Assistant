use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};

use word_guess_assistant_lib::codec::{
    game_message::Data, BlankGuessAction, ChatMessage, GameMessage, GamePhase, JoinRequest,
    ProtobufCodec, VoteAction,
};

struct BotConfig {
    id: &'static str,
    name: &'static str,
    avatar: &'static str,
    description_templates: &'static [&'static str],
}

const BOTS: &[BotConfig] = &[
    BotConfig {
        id: "bot_alpha",
        name: "🤖 AI特工·阿法",
        avatar: "/avatar/cyber/hacker.svg",
        description_templates: &[
            "根据我的初步观察，这个事物在日常工作或生活中非常普遍。",
            "我拿到的词汇感觉大家经常会接触，属于不可或缺的存在！",
            "大家别看我，我这边的线索感觉非常正统，绝对是好人！",
        ],
    },
    BotConfig {
        id: "bot_beta",
        name: "🐼 AI特工·贝塔",
        avatar: "/avatar/animals/panda.svg",
        description_templates: &[
            "我这边的词汇特征十分鲜明，而且用途很广泛哦！",
            "我的描述很简单：它通常能带来便利或者满足某种需求~",
            "我也附议前面的看法，感觉整体调性非常契合！",
        ],
    },
    BotConfig {
        id: "bot_gamma",
        name: "🦊 AI特工·伽马",
        avatar: "/avatar/animals/fox.svg",
        description_templates: &[
            "我的这个词不仅实用，而且在特定场景下辨识度非常高！",
            "我感觉周围有人的发言有点含糊其辞哦，是不是卧底在划水？",
            "本人的词汇非常接地气，听完大家的描述我心中已有数。",
        ],
    },
    BotConfig {
        id: "bot_delta",
        name: "🐱 AI特工·德尔塔",
        avatar: "/avatar/animals/cat.svg",
        description_templates: &[
            "这个东西在现代社会很普及，大部分人都使用或体验过。",
            "词义清晰明确，我的描述完全站得住脚，请大家放心！",
        ],
    },
    BotConfig {
        id: "bot_epsilon",
        name: "⚡ AI特工·艾普西龙",
        avatar: "/avatar/cyber/cyborg.svg",
        description_templates: &[
            "数据分析显示，该词汇属于非常经典的概念，毫无歧义。",
            "我正在交叉比对所有玩家的发信模式，静待下一轮线索。",
        ],
    },
    BotConfig {
        id: "bot_zeta",
        name: "🕵️ AI特工·泽塔",
        avatar: "/avatar/characters/detective.svg",
        description_templates: &[
            "王牌侦探登场，我的词汇属性非常纯正，好人阵营稳了。",
            "某些细节差异已经浮出水面，卧底请好自为之！",
        ],
    },
    BotConfig {
        id: "bot_eta",
        name: "🥷 AI特工·伊塔",
        avatar: "/avatar/characters/ninja.svg",
        description_templates: &[
            "潜伏暗影，我的线索词具备强烈的场景关联度。",
            "不要轻易试探我的底牌，我持有的是关键信息。",
        ],
    },
    BotConfig {
        id: "bot_theta",
        name: "🧙 AI特工·西塔",
        avatar: "/avatar/characters/wizard.svg",
        description_templates: &[
            "奥术占卜表明，本局词汇关联度极高，大家需细心辨别。",
            "我的词汇蕴含着某种自然或人为规律，大家体会一下。",
        ],
    },
    BotConfig {
        id: "bot_iota",
        name: "🐰 AI特工·约塔",
        avatar: "/avatar/animals/rabbit.svg",
        description_templates: &[
            "小巧玲珑，我的词汇给大家的第一印象总是很亲切！",
            "我也持赞同态度，顺着前几位的方向继续推进就好啦。",
        ],
    },
    BotConfig {
        id: "bot_kappa",
        name: "🐻 AI特工·卡帕",
        avatar: "/avatar/animals/bear.svg",
        description_templates: &[
            "沉稳如山，这个词汇在现实中有着不可替代的分量。",
            "我的描述绝不弄虚作假，大家尽管验证！",
        ],
    },
    BotConfig {
        id: "bot_lambda",
        name: "🦸 AI特工·拉姆达",
        avatar: "/avatar/characters/hero.svg",
        description_templates: &[
            "为了正义与好人阵营，我的描述将直击本质！",
            "卧底不要挣扎了，大家仔细对比词义细节即可破局。",
        ],
    },
];

async fn run_bot(config: &'static BotConfig, port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    println!(">>> [{}] 正在连接房主服务器: {} ...", config.name, addr);

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ [{}] 无法连接到 {}: {}", config.name, addr, e);
            eprintln!("   请确保房主已经在该端口（默认 8888）成功创建并开启了房间！");
            return;
        }
    };

    println!("✅ [{}] 成功建立 TCP 握手！正在发送入房认证包...", config.name);

    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, ProtobufCodec::new());
    let mut writer = FramedWrite::new(write_half, ProtobufCodec::new());

    // 1. 发送 JoinRequest
    let join_msg = GameMessage {
        data: Some(Data::JoinReq(JoinRequest {
            player_id: config.id.to_string(),
            player_name: config.name.to_string(),
            avatar: config.avatar.to_string(),
        })),
    };

    if let Err(e) = writer.send(join_msg).await {
        eprintln!("❌ [{}] 发送入房请求失败: {}", config.name, e);
        return;
    }

    let mut current_role = String::new();
    let mut current_word = String::new();
    let mut has_described = false;

    // 2. 消息接收与状态机循环响应
    while let Some(msg_res) = reader.next().await {
        match msg_res {
            Ok(msg) => {
                if let Some(data) = msg.data {
                    match data {
                        Data::Identity(id_info) => {
                            current_role = id_info.role.clone();
                            current_word = id_info.word.clone();
                            println!(
                                "📜 [{}] 收到私密身份分配 -> 阵营: [{}], 密词: [{}], 提示: [{}]",
                                config.name, id_info.role, id_info.word, id_info.hint
                            );
                        }
                        Data::Chat(chat) => {
                            if !chat.is_system && chat.sender_id != config.id {
                                println!("💬 [{}] 收到发言: {}", config.name, chat.text);
                            }
                        }
                        Data::SyncPhase(phase_info) => {
                            let phase = phase_info.new_phase;
                            println!(
                                "🔄 [{}] 收到阶段流转同步 -> 阶段代码: {}, 说明: {}",
                                config.name, phase, phase_info.extra_info
                            );

                            if phase == GamePhase::Preparing as i32 {
                                has_described = false;
                            } else if phase == GamePhase::Description as i32 {
                                if !has_described {
                                    // 仅第 1 轮描述阶段发言：模拟思考（1.0 ~ 3.0 秒随机错开），如同真人打字排队
                                    has_described = true;
                                    let delay = 1000 + ((config.id.as_bytes().iter().map(|b| *b as u64).sum::<u64>()) % 2000);
                                    tokio::time::sleep(Duration::from_millis(delay)).await;
                                    let template_idx = (config.id.as_bytes().iter().map(|b| *b as usize).sum::<usize>()) % config.description_templates.len();
                                    let text = config.description_templates[template_idx];

                                    println!("✍️  [{}] 正在提交第 1 轮线索描述: \"{}\"", config.name, text);
                                    let desc_msg = GameMessage {
                                        data: Some(Data::Chat(ChatMessage {
                                            sender_id: config.id.to_string(),
                                            text: text.to_string(),
                                            is_system: false,
                                        })),
                                    };
                                    let _ = writer.send(desc_msg).await;
                                } else {
                                    println!("☀️ [{}] 收到白天通知：根据规则后续白天无需重复发言，保持静默回顾。", config.name);
                                }
                            } else if phase == GamePhase::Voting as i32 {
                                // 投票阶段：模拟思考（1.2 ~ 3.5 秒随机错开），观察并分别投出公投票
                                let delay = 1200 + ((config.id.as_bytes().iter().map(|b| *b as u64).sum::<u64>()) % 2300);
                                tokio::time::sleep(Duration::from_millis(delay)).await;

                                // 分散票型：避免所有人集火同一目标导致游戏过早结束，形成 3:3:2 等富有悬念的票型分布
                                let target = match config.id {
                                    "bot_alpha" | "bot_beta" | "bot_gamma" => "bot_delta",
                                    "bot_delta" | "bot_epsilon" => "bot_alpha",
                                    _ => "bot_beta",
                                };
                                println!("🗳️  [{}] 正在提交公投票选 -> 目标: [{}]", config.name, target);
                                let vote_msg = GameMessage {
                                    data: Some(Data::Vote(VoteAction {
                                        voter_id: config.id.to_string(),
                                        target_id: target.to_string(),
                                    })),
                                };
                                let _ = writer.send(vote_msg).await;
                            } else if phase == GamePhase::BlankGuess as i32 {
                                println!(
                                    "✨ [{}] 收到白板猜词特别阶段通知。我的真实角色是: [{}]",
                                    config.name, current_role
                                );
                                if current_role == "BLANK" {
                                    tokio::time::sleep(Duration::from_millis(1500)).await;
                                    println!("✨ [{}] 为被公投出局的白板，正在提交模拟双词猜词...", config.name);
                                    let guess_msg = GameMessage {
                                        data: Some(Data::BlankGuess(BlankGuessAction {
                                            player_id: config.id.to_string(),
                                            word_a: "水果".to_string(),
                                            word_b: "蔬菜".to_string(),
                                        })),
                                    };
                                    let _ = writer.send(guess_msg).await;
                                }
                            } else if phase == GamePhase::NightAction as i32 {
                                println!(
                                    "🌙 [{}] 收到黑夜流转通知（黑夜暗杀阶段开始）。我的真实角色是: [{}], 密词: [{}]",
                                    config.name, current_role, current_word
                                );
                                if current_role == "SPY" {
                                    println!("   🗡️ [{}] 为卧底特工：保持静默潜伏，等待房主/玩家在界面体验申请刀人...", config.name);
                                } else {
                                    println!("   🛡️ [{}] 为好人/天使/白板：黑夜闭眼中，等待天亮...", config.name);
                                }
                            } else if phase == GamePhase::Settlement as i32 {
                                println!(
                                    "🏆 [{}] 游戏进入终局战报结算，我的真实角色是: {}，我的词是: {}",
                                    config.name, current_role, current_word
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️ [{}] 连接断开或异常: {}", config.name, e);
                break;
            }
        }
    }

    println!("🚪 [{}] 已退出与房主的连接。", config.name);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("🤖 Word Guess Assistant - 虚拟特工测试客户端 (Mock Bots)");
        println!("用法: pnpm mock:bots [端口号] [机器人数量]");
        println!("参数说明:");
        println!("  [端口号]       房主监听的局域网端口，默认 8888");
        println!("  [机器人数量]   启动的 AI 特工数量 (1 ~ 11)，默认 8");
        println!("示例:");
        println!("  pnpm mock:bots          # 连接 127.0.0.1:8888，启动 8 名特工");
        println!("  pnpm mock:bots 8888 4   # 连接 127.0.0.1:8888，启动 4 名特工");
        println!("  pnpm test:bots          # 纯终端自动化闭环集成对决测试");
        return;
    }

    let port = args
        .get(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8888);

    // 默认启动 8 名虚拟陪玩机器人（加上房主自身恰好为 9 人局，完美体验白天公投、黑夜盲动暗杀与后续轮次）
    let bot_count = args
        .get(2)
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(8)
        .clamp(1, BOTS.len());

    println!("============================================================");
    println!("🤖 Word Guess Assistant - 虚拟测试客户端启动器 (Mock Bots)");
    println!("============================================================");
    println!("🎯 目标地址: 127.0.0.1:{}", port);
    println!("👥 启动虚拟特工数量: {} 名", bot_count);
    println!("💡 算上房主自身，房间总人数将达到: {} 名 (多阵营格局，支持完整体验夜晚刀人)", bot_count + 1);
    println!("------------------------------------------------------------");

    let mut handles = Vec::new();
    for i in 0..bot_count {
        let bot = &BOTS[i];
        let handle = tokio::spawn(run_bot(bot, port));
        handles.push(handle);
        // 轻微错开连接时间，确保网络握手有序稳定
        tokio::time::sleep(Duration::from_millis(80)).await;
    }

    for h in handles {
        let _ = h.await;
    }
}
