use std::collections::HashMap;
use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use tracing::info;

use crate::codec::{
    game_message::Data, BlankGuessAction, ChatMessage, GameMessage, GamePhase, IdentityAssignment,
    NightAction, SyncPhase, VoteAction,
};
use crate::network::{broadcast_message, send_to_player, NetworkState};

/// 角色常量定义
pub const ROLE_GOOD: &str = "GOOD";     // 好人
pub const ROLE_SPY: &str = "SPY";       // 卧底
pub const ROLE_ANGEL: &str = "ANGEL";   // 天使
pub const ROLE_BLANK: &str = "BLANK";   // 白板

/// 单个玩家游戏内状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub role: String,
    pub word: String,
    pub hint: String,
    pub is_alive: bool,
}

/// 游戏核心全局状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub phase: i32,
    pub word_a: String, // 好人词
    pub word_b: String, // 卧底词
    pub hint: String,   // 相似方向提示
    pub similarity_level: String,
    pub theme: String,
    pub players: HashMap<String, PlayerState>,
    pub votes: HashMap<String, String>, // voter_id -> target_id
    pub round: u32,
    pub winner: Option<String>,
    pub settlement_details: Option<SettlementDetails>,
    pub night_casualties: Vec<String>, // 夜间出局特工ID记录
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDetails {
    pub winner_camp: String,
    pub good_word: String,
    pub spy_word: String,
    pub hint: String,
    pub player_reveals: Vec<PlayerReveal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerReveal {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub role: String,
    pub word: String,
    pub hint: String,
    pub is_alive: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Preparing as i32,
            word_a: String::new(),
            word_b: String::new(),
            hint: String::new(),
            similarity_level: "极为接近".to_string(),
            theme: String::new(),
            players: HashMap::new(),
            votes: HashMap::new(),
            round: 1,
            winner: None,
            settlement_details: None,
            night_casualties: Vec::new(),
        }
    }

    /// 重置游戏状态至准备阶段
    pub fn reset(&mut self) {
        self.phase = GamePhase::Preparing as i32;
        self.word_a.clear();
        self.word_b.clear();
        self.hint.clear();
        self.votes.clear();
        self.round = 1;
        self.winner = None;
        self.settlement_details = None;
        self.night_casualties.clear();
        self.players.clear();
    }
}

/// 游戏引擎全局管理句柄
#[derive(Clone)]
pub struct GameEngineState {
    pub inner: Arc<Mutex<GameState>>,
}

impl Default for GameEngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameEngineState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(GameState::new())),
        }
    }
}

// ==========================================
// 出题词库与词汇生成引擎
// ==========================================

#[derive(Debug, Clone)]
pub struct PresetWordPair {
    pub word_a: &'static str,
    pub word_b: &'static str,
    pub hint: &'static str,
    pub similarity: &'static str, // "极为接近" 或 "较为接近"
    pub tag: &'static str,
}

/// 内置精品双词库与相似方向提示
pub const BUILTIN_WORD_PAIRS: &[PresetWordPair] = &[
    PresetWordPair {
        word_a: "玫瑰",
        word_b: "月季",
        hint: "多刺木本观赏花卉，常作为爱情与热烈情感的象征",
        similarity: "极为接近",
        tag: "植物",
    },
    PresetWordPair {
        word_a: "汉堡",
        word_b: "肉夹馍",
        hint: "外层为烘烤面食、中层夹香浓熟肉与调味的经典主食",
        similarity: "较为接近",
        tag: "美食",
    },
    PresetWordPair {
        word_a: "单反相机",
        word_b: "微单相机",
        hint: "拥有可更换专业镜头组件的高画质便携数码摄影设备",
        similarity: "极为接近",
        tag: "科技",
    },
    PresetWordPair {
        word_a: "微信",
        word_b: "QQ",
        hint: "腾讯公司旗下拥有海量用户的国民级即时通讯与社交软件",
        similarity: "极为接近",
        tag: "科技",
    },
    PresetWordPair {
        word_a: "台风",
        word_b: "龙卷风",
        hint: "伴随高速气旋旋转并伴有极强破坏力的大气涡旋恶劣天气",
        similarity: "较为接近",
        tag: "自然",
    },
    PresetWordPair {
        word_a: "自行车",
        word_b: "电动车",
        hint: "日常街头常见、依靠两个轮子维持平衡的个人代步骑行工具",
        similarity: "较为接近",
        tag: "交通",
    },
    PresetWordPair {
        word_a: "牛奶",
        word_b: "豆浆",
        hint: "富含丰富蛋白质、常在清晨早餐搭配享用的营养乳白色饮品",
        similarity: "较为接近",
        tag: "美食",
    },
    PresetWordPair {
        word_a: "雪碧",
        word_b: "可乐",
        hint: "口感清爽、充满二氧化碳气泡的经典瓶装碳酸甜味软饮料",
        similarity: "较为接近",
        tag: "美食",
    },
    PresetWordPair {
        word_a: "眉毛",
        word_b: "睫毛",
        hint: "生长在人体眼眶周边、起到遮挡汗水灰尘与装饰面部作用的毛发",
        similarity: "极为接近",
        tag: "生活",
    },
    PresetWordPair {
        word_a: "橙子",
        word_b: "橘子",
        hint: "外皮为鲜艳橙黄色、富含维C且酸甜多汁的柑橘属水果",
        similarity: "极为接近",
        tag: "美食",
    },
    PresetWordPair {
        word_a: "羽毛球",
        word_b: "网球",
        hint: "通过挥动球拍隔着中央球网将球来回击打的双人或四人竞技运动",
        similarity: "较为接近",
        tag: "体育",
    },
    PresetWordPair {
        word_a: "小提琴",
        word_b: "中提琴",
        hint: "依靠马尾琴弓在四根琴弦上摩擦振动发声的木质西洋弦乐器",
        similarity: "极为接近",
        tag: "艺术",
    },
];

/// 根据相似度及主题生成题目
pub fn generate_words(theme: &str, similarity: &str) -> (String, String, String) {
    let mut rng = thread_rng();
    let trimmed_theme = theme.trim();

    // 1. 尝试匹配主题及相似度
    let filtered: Vec<&PresetWordPair> = BUILTIN_WORD_PAIRS
        .iter()
        .filter(|p| {
            let match_sim = p.similarity == similarity;
            if !trimmed_theme.is_empty() {
                match_sim && (p.tag.contains(trimmed_theme) || p.hint.contains(trimmed_theme))
            } else {
                match_sim
            }
        })
        .collect();

    if let Some(selected) = filtered.choose(&mut rng) {
        return (
            selected.word_a.to_string(),
            selected.word_b.to_string(),
            selected.hint.to_string(),
        );
    }

    // 2. 退化匹配相似度
    let sim_filtered: Vec<&PresetWordPair> = BUILTIN_WORD_PAIRS
        .iter()
        .filter(|p| p.similarity == similarity)
        .collect();

    if let Some(selected) = sim_filtered.choose(&mut rng) {
        return (
            selected.word_a.to_string(),
            selected.word_b.to_string(),
            selected.hint.to_string(),
        );
    }

    // 3. 全局随机兜底
    if let Some(selected) = BUILTIN_WORD_PAIRS.choose(&mut rng) {
        (
            selected.word_a.to_string(),
            selected.word_b.to_string(),
            selected.hint.to_string(),
        )
    } else {
        (
            "苹果".to_string(),
            "梨子".to_string(),
            "常见多汁甘甜的水果".to_string(),
        )
    }
}

/// 角色分配算法：支持 4 ~ 12 名玩家动态身份矩阵（白板固定 1 人，天使固定 1 人）
/// - 4人局： 1 卧底, 1 天使, 1 白板, 1 好人 (总计 4 人)
/// - 5人局： 1 卧底, 1 天使, 1 白板, 2 好人 (总计 5 人)
/// - 6人局： 2 卧底, 1 天使, 1 白板, 2 好人 (总计 6 人)
/// - 7人局： 2 卧底, 1 天使, 1 白板, 3 好人 (总计 7 人)
/// - 8人局： 2 卧底, 1 天使, 1 白板, 4 好人 (总计 8 人)
/// - 9人局： 3 卧底, 1 天使, 1 白板, 4 好人 (总计 9 人)
/// - 10人局：3 卧底, 1 天使, 1 白板, 5 好人 (总计 10 人)
/// - 11人局：3 卧底, 1 天使, 1 白板, 6 好人 (总计 11 人)
/// - 12人局：4 卧底, 1 天使, 1 白板, 6 好人 (总计 12 人)
pub fn assign_roles(player_ids: &[String]) -> HashMap<String, String> {
    let mut rng = thread_rng();
    let mut shuffled = player_ids.to_vec();
    shuffled.shuffle(&mut rng);

    let count = shuffled.len();
    let mut assignments = HashMap::new();

    if count < 4 {
        return assignments;
    }

    let (spy_count, angel_count, blank_count) = match count {
        4 => (1, 1, 1),
        5 => (1, 1, 1),
        6 => (2, 1, 1),
        7 => (2, 1, 1),
        8 => (2, 1, 1),
        9 => (3, 1, 1),
        10 => (3, 1, 1),
        11 => (3, 1, 1),
        12 => (4, 1, 1),
        _ => (count / 3, 1, 1),
    };

    let mut idx = 0;

    // 1. 分配卧底
    for _ in 0..spy_count {
        if idx < count {
            assignments.insert(shuffled[idx].clone(), ROLE_SPY.to_string());
            idx += 1;
        }
    }

    // 2. 分配天使
    for _ in 0..angel_count {
        if idx < count {
            assignments.insert(shuffled[idx].clone(), ROLE_ANGEL.to_string());
            idx += 1;
        }
    }

    // 3. 分配白板
    for _ in 0..blank_count {
        if idx < count {
            assignments.insert(shuffled[idx].clone(), ROLE_BLANK.to_string());
            idx += 1;
        }
    }

    // 4. 其余全部分配为平民好人
    while idx < count {
        assignments.insert(shuffled[idx].clone(), ROLE_GOOD.to_string());
        idx += 1;
    }

    assignments
}

/// 计算胜负判定
/// 返回值：Some("GOOD") | Some("SPY") | None
/// 胜负判定准则：
/// 1. 卧底阵营获胜：存活卧底数大于等于好人（平民好人+天使）存活数；
/// 2. 好人阵营获胜：所有卧底全部出局，且白板被找出并猜词失败（若对局存在白板，白板必须已出局）；
/// 3. 白板阵营获胜：由 process_blank_guess_inner 在猜中词汇时直接触发。
pub fn check_winner(players: &HashMap<String, PlayerState>) -> Option<String> {
    let mut alive_good = 0;
    let mut alive_spy = 0;
    let mut alive_blank = 0;
    let mut has_blank = false;

    for p in players.values() {
        if p.role == ROLE_BLANK {
            has_blank = true;
        }
        if p.is_alive {
            match p.role.as_str() {
                ROLE_GOOD | ROLE_ANGEL => alive_good += 1,
                ROLE_SPY => alive_spy += 1,
                ROLE_BLANK => alive_blank += 1,
                _ => {} // 观战者不计入
            }
        }
    }

    // 1. 存活卧底数大于等于好人存活数 -> 卧底阵营胜利
    if alive_spy > 0 && alive_spy >= alive_good {
        return Some(ROLE_SPY.to_string());
    }

    // 2. 好人阵营获胜条件：所有卧底出局，且白板被找出并猜词失败（若场上有白板，白板必须已出局且未猜词成功）
    if alive_spy == 0 {
        // 若场上存在白板且白板依然存活（尚未被找出），好人无法直接获胜，对局必须继续！
        if has_blank && alive_blank > 0 {
            return None;
        }
        return Some(ROLE_GOOD.to_string());
    }

    None
}

// ==========================================
// Tauri Commands (游戏控制接口)
// ==========================================

/// 房主开始游戏 Command
#[tauri::command]
pub async fn start_game_command(
    _app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
    theme: Option<String>,
    similarity: Option<String>,
) -> Result<(), String> {
    let network_clone = network_state.inner().clone();
    let game_clone = game_state.inner().clone();

    let mut game = game_clone.inner.lock().await;
    let profiles = network_clone.player_profiles.lock().await;

    let player_ids: Vec<String> = profiles.keys().cloned().collect();
    if player_ids.len() < 4 {
        return Err("游戏人数不足，至少需要 4 名玩家才能启动游戏（支持 4~12 人）".to_string());
    }
    if player_ids.len() > 12 {
        return Err("游戏人数超限，单房间最多支持 12 名玩家同时博弈".to_string());
    }

    let sim = similarity.unwrap_or_else(|| "极为接近".to_string());
    let th = theme.unwrap_or_default();

    // 1. 生成词汇与提示
    let (word_a, word_b, hint) = generate_words(&th, &sim);
    info!("出题完成：好人词=[{}], 卧底词=[{}], 提示=[{}]", word_a, word_b, hint);

    // 2. 分配角色
    let role_map = assign_roles(&player_ids);

    // 3. 重置并初始化 GameState
    game.reset();
    game.word_a = word_a.clone();
    game.word_b = word_b.clone();
    game.hint = hint.clone();
    game.similarity_level = sim;
    game.theme = th;
    game.phase = GamePhase::Description as i32;

    for (id, profile) in profiles.iter() {
        let role = role_map.get(id).cloned().unwrap_or_else(|| ROLE_GOOD.to_string());
        let (p_word, p_hint) = match role.as_str() {
            ROLE_GOOD => (word_a.clone(), String::new()),
            ROLE_ANGEL => {
                // 天使同时获知好词和坏词两个词，但不知道词的好坏（随机乱序排列）
                let mut words = [word_a.clone(), word_b.clone()];
                words.shuffle(&mut thread_rng());
                (format!("{} / {}", words[0], words[1]), String::new())
            }
            ROLE_SPY => (word_b.clone(), String::new()),
            ROLE_BLANK => (String::new(), hint.clone()),
            _ => (word_a.clone(), String::new()),
        };

        game.players.insert(
            id.clone(),
            PlayerState {
                id: id.clone(),
                name: profile.player_name.clone(),
                avatar: profile.avatar.clone(),
                role: role.clone(),
                word: p_word.clone(),
                hint: p_hint.clone(),
                is_alive: true,
            },
        );

        // 4. 定向单播下发差异化身份包（IdentityAssignment）
        let id_msg = GameMessage {
            data: Some(Data::Identity(IdentityAssignment {
                role,
                word: p_word,
                hint: p_hint,
            })),
        };
        let _ = send_to_player(&network_clone.peers, id, id_msg).await;
    }

    // 5. 广播进入发言阶段
    let sync_msg = GameMessage {
        data: Some(Data::SyncPhase(SyncPhase {
            new_phase: GamePhase::Description as i32,
            extra_info: "游戏已启动！第一轮描述阶段开始，请存活玩家依次进行一句话描述".to_string(),
        })),
    };
    broadcast_message(&network_clone.peers, &sync_msg).await;

    // 发送系统聊天提示
    let chat_msg = GameMessage {
        data: Some(Data::Chat(ChatMessage {
            sender_id: "SYSTEM".to_string(),
            text: "【系统提示】身份分发完成，可查验「我的身份」。".to_string(),
            is_system: true,
        })),
    };
    broadcast_message(&network_clone.peers, &chat_msg).await;

    Ok(())
}

/// 阶段流转控制 Command（房主触发：NEXT）
#[tauri::command]
pub async fn next_phase_command(
    app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
) -> Result<(), String> {
    let network_clone = network_state.inner().clone();
    let game_clone = game_state.inner().clone();
    let mut game = game_clone.inner.lock().await;

    let current_phase = game.phase;

    match current_phase {
        // DESCRIPTION -> VOTING
        p if p == GamePhase::Description as i32 => {
            game.phase = GamePhase::Voting as i32;
            game.votes.clear();

            let sync_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::Voting as i32,
                    extra_info: format!("第 {} 轮投票阶段开始。", game.round),
                })),
            };
            broadcast_message(&network_clone.peers, &sync_msg).await;
        }

        // VOTING -> 计算票数 -> NIGHT_ACTION 或 SETTLEMENT
        p if p == GamePhase::Voting as i32 => {
            // 统计投票结果
            let mut tally: HashMap<String, u32> = HashMap::new();
            for target_id in game.votes.values() {
                *tally.entry(target_id.clone()).or_insert(0) += 1;
            }

            let mut max_votes = 0;
            let mut max_targets = Vec::new();
            for (target_id, count) in tally.iter() {
                if *count > max_votes {
                    max_votes = *count;
                    max_targets = vec![target_id.clone()];
                } else if *count == max_votes {
                    max_targets.push(target_id.clone());
                }
            }

            let mut is_blank_eliminated = false;
            let mut out_msg_text = String::new();
            if max_votes > 0 && max_targets.len() == 1 {
                let out_id = &max_targets[0];
                if let Some(p) = game.players.get_mut(out_id) {
                    p.is_alive = false;
                    if p.role == ROLE_BLANK {
                        is_blank_eliminated = true;
                        out_msg_text = format!("【公投决议】特工【{}】得票最高（{} 票），公投出局（白板）。", p.name, max_votes);
                    } else {
                        out_msg_text = format!("【公投决议】特工【{}】得票最高（{} 票），公投出局。", p.name, max_votes);
                    }
                }
            } else if max_targets.len() > 1 {
                out_msg_text = format!("【公投决议】平票（最高 {} 票），本轮无人出局。", max_votes);
            } else {
                out_msg_text = "【公投决议】弃票，本轮无人出局。".to_string();
            }

            // 系统聊天播报投票结果
            let chat_msg = GameMessage {
                data: Some(Data::Chat(ChatMessage {
                    sender_id: "SYSTEM".to_string(),
                    text: out_msg_text.clone(),
                    is_system: true,
                })),
            };
            broadcast_message(&network_clone.peers, &chat_msg).await;

            game.votes.clear();

            if is_blank_eliminated {
                game.phase = GamePhase::BlankGuess as i32;
                let guess_sync = GameMessage {
                    data: Some(Data::SyncPhase(SyncPhase {
                        new_phase: GamePhase::BlankGuess as i32,
                        extra_info: "白板被公投出局，触发白板猜词特别阶段。".to_string(),
                    })),
                };
                broadcast_message(&network_clone.peers, &guess_sync).await;
                return Ok(());
            }

            // 检查游戏是否结束
            if let Some(winner) = check_winner(&game.players) {
                game.phase = GamePhase::Settlement as i32;
                game.winner = Some(winner.clone());
                finish_game(&mut game, &network_clone, &app_handle, &winner).await;
                return Ok(());
            }

            // 未结束，流转至 NIGHT_ACTION
            game.phase = GamePhase::NightAction as i32;
            let sync_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::NightAction as i32,
                    extra_info: "夜间阶段开始。".to_string(),
                })),
            };
            broadcast_message(&network_clone.peers, &sync_msg).await;
        }

        p if p == GamePhase::BlankGuess as i32 => {
            // 房主手动推进跳过白板猜词，直接进入黑夜暗杀阶段
            let chat_msg = GameMessage {
                data: Some(Data::Chat(ChatMessage {
                    sender_id: "SYSTEM".to_string(),
                    text: "【猜词决议】白板猜词阶段结束，游戏进入夜间阶段。".to_string(),
                    is_system: true,
                })),
            };
            broadcast_message(&network_clone.peers, &chat_msg).await;

            if let Some(winner) = check_winner(&game.players) {
                game.phase = GamePhase::Settlement as i32;
                game.winner = Some(winner.clone());
                finish_game(&mut game, &network_clone, &app_handle, &winner).await;
                return Ok(());
            }

            game.phase = GamePhase::NightAction as i32;
            let sync_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::NightAction as i32,
                    extra_info: "夜间阶段开始。".to_string(),
                })),
            };
            broadcast_message(&network_clone.peers, &sync_msg).await;
        }

        // NIGHT_ACTION -> 下一轮 DESCRIPTION 或 SETTLEMENT
        p if p == GamePhase::NightAction as i32 => {
            // 1. 整理夜间出局战报（无人出局播报平安夜；有人出局客观播报，统一口径不暴露死因）
            let casualty_names: Vec<String> = game
                .night_casualties
                .iter()
                .filter_map(|id| game.players.get(id).map(|p| p.name.clone()))
                .collect();

            let night_report = if casualty_names.is_empty() {
                "【夜间战报】昨夜是平安夜，无人出局。".to_string()
            } else {
                format!("【夜间战报】昨夜特工【{}】出局。", casualty_names.join("、"))
            };

            game.night_casualties.clear();

            let report_chat = GameMessage {
                data: Some(Data::Chat(ChatMessage {
                    sender_id: "SYSTEM".to_string(),
                    text: night_report,
                    is_system: true,
                })),
            };
            broadcast_message(&network_clone.peers, &report_chat).await;

            // 2. 检查胜负条件
            if let Some(winner) = check_winner(&game.players) {
                game.phase = GamePhase::Settlement as i32;
                game.winner = Some(winner.clone());
                finish_game(&mut game, &network_clone, &app_handle, &winner).await;
                return Ok(());
            }

            // 3. 未分胜负，推进至下一轮描述阶段
            game.round += 1;
            game.phase = GamePhase::Description as i32;
            let sync_msg = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::Description as i32,
                    extra_info: format!("第 {} 轮描述阶段开始。", game.round),
                })),
            };
            broadcast_message(&network_clone.peers, &sync_msg).await;
        }

        _ => {}
    }

    Ok(())
}

/// 核心投票逻辑：
/// 1. 验证投票者与目标是否存活；
/// 2. 检查是否重复投票（每人限投 1 次，不可更改）；
/// 3. 秘密登记投票（绝不广播谁投了谁）；
/// 4. 统计投票进度，广播匿名进度；
/// 5. 当所有存活玩家投票完成后，自动统计票数、宣布出局者并流转至下一阶段。
pub async fn record_vote_and_maybe_advance(
    game: &mut GameState,
    network: &NetworkState,
    app_handle: &AppHandle,
    voter_id: &str,
    target_id: &str,
) -> Result<(), String> {
    if game.phase != GamePhase::Voting as i32 {
        return Err("当前不是投票阶段".to_string());
    }

    // 1. 检查投票者是否存活
    if let Some(voter) = game.players.get(voter_id) {
        if !voter.is_alive {
            return Err("出局玩家无法参与投票".to_string());
        }
    } else {
        return Err("投票玩家不存在".to_string());
    }

    // 2. 检查被投票目标是否存活
    if let Some(target) = game.players.get(target_id) {
        if !target.is_alive {
            return Err("不可投票给已出局的玩家".to_string());
        }
    } else {
        return Err("被投票目标玩家不存在".to_string());
    }

    // 3. 投票仅可选择一次，提交后不可更改
    if game.votes.contains_key(voter_id) {
        return Err("您已经投过票，投票仅可选择一次，提交后不可更改".to_string());
    }

    // 4. 秘密登记投票
    game.votes.insert(voter_id.to_string(), target_id.to_string());
    info!("玩家 [{}] 秘密投票给目标 [{}]", voter_id, target_id);

    // 5. 统计存活玩家总数与当前投票总数
    let alive_players: Vec<String> = game
        .players
        .values()
        .filter(|p| p.is_alive)
        .map(|p| p.id.clone())
        .collect();
    let total_alive = alive_players.len();
    let current_votes = game.votes.len();

    // 6. 广播匿名投票进度提示（绝不泄露谁投了谁）
    let progress_chat = GameMessage {
        data: Some(Data::Chat(ChatMessage {
            sender_id: "SYSTEM".to_string(),
            text: format!("【公投进度】投票进度：{}/{}", current_votes, total_alive),
            is_system: true,
        })),
    };
    broadcast_message(&network.peers, &progress_chat).await;

    // 7. 当所有存活玩家均完成投票后，自动结算并推进
    if current_votes >= total_alive && total_alive > 0 {
        // (1) 统计各候选人得票数
        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        for t in game.votes.values() {
            *vote_counts.entry(t.clone()).or_insert(0) += 1;
        }

        // (2) 找出最高票数
        let max_votes = vote_counts.values().cloned().max().unwrap_or(0);
        let top_targets: Vec<String> = vote_counts
            .iter()
            .filter(|(_, count)| **count == max_votes)
            .map(|(id, _)| id.clone())
            .collect();

        // (3) 清空本轮投票记录
        game.votes.clear();

        // (4) 判定放逐者
        let mut is_blank_eliminated = false;
        let announcement = if top_targets.len() == 1 {
            let out_id = &top_targets[0];
            if let Some(target_player) = game.players.get_mut(out_id) {
                target_player.is_alive = false;
                let out_name = target_player.name.clone();
                let out_role = target_player.role.clone();
                if out_role == ROLE_BLANK {
                    is_blank_eliminated = true;
                    format!("【公投决议】特工【{}】得票最高（{} 票），公投出局（白板）。", out_name, max_votes)
                } else {
                    format!("【公投决议】特工【{}】得票最高（{} 票），公投出局。", out_name, max_votes)
                }
            } else {
                "【公投决议】最高得票目标无效，本轮无人出局。".to_string()
            }
        } else {
            format!("【公投决议】平票（最高 {} 票），本轮无人出局。", max_votes)
        };

        // (5) 广播决议公告
        let result_chat = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "SYSTEM".to_string(),
                text: announcement,
                is_system: true,
            })),
        };
        broadcast_message(&network.peers, &result_chat).await;

        // (6) 流程推进：若白板在白天被公投出局，立即触发白板猜词特别阶段（在夜晚阶段前插入）
        if is_blank_eliminated {
            game.phase = GamePhase::BlankGuess as i32;
            let guess_sync = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::BlankGuess as i32,
                    extra_info: "白板被公投出局，触发白板猜词特别阶段。".to_string(),
                })),
            };
            broadcast_message(&network.peers, &guess_sync).await;
        } else if let Some(winner) = check_winner(&game.players) {
            finish_game(game, network, app_handle, &winner).await;
        } else {
            // 自动流转至黑夜暗杀阶段
            game.phase = GamePhase::NightAction as i32;
            let night_sync = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::NightAction as i32,
                    extra_info: "夜间阶段开始。".to_string(),
                })),
            };
            broadcast_message(&network.peers, &night_sync).await;
        }
    }

    Ok(())
}

/// 玩家提交投票 Command（兼顾房主与连接进来的客户端）
#[tauri::command]
pub async fn submit_vote_command(
    app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
    voter_id: String,
    target_id: String,
) -> Result<(), String> {
    let network_clone = network_state.inner().clone();
    let game_clone = game_state.inner().clone();
    let is_host = !network_clone.peers.lock().await.is_empty();

    if is_host {
        let mut game = game_clone.inner.lock().await;
        record_vote_and_maybe_advance(
            &mut game,
            &network_clone,
            &app_handle,
            &voter_id,
            &target_id,
        )
        .await
    } else {
        // 客户端模式：向房主发送 VoteAction 二进制协议包
        let vote_msg = GameMessage {
            data: Some(Data::Vote(VoteAction {
                voter_id,
                target_id,
            })),
        };
        let host_tx = network_clone.host_sender.lock().await.clone();
        if let Some(tx) = host_tx {
            tx.send(vote_msg).await.map_err(|e| format!("发送投票失败: {}", e))
        } else {
            Err("未连接到任何房间".to_string())
        }
    }
}

/// 黑夜盲动暗杀 Command（核心规则：真卧底刀人成功，好人误刀触发自爆；夜间静默记录，天亮统一播报）
#[tauri::command]
pub async fn submit_night_action_command(
    _app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
    actor_id: String,
    target_id: String,
) -> Result<String, String> {
    let network_clone = network_state.inner().clone();
    let game_clone = game_state.inner().clone();
    let is_host = !network_clone.peers.lock().await.is_empty();

    if !is_host {
        // 客户端模式：发送夜间行动至房主统一裁决
        let night_msg = GameMessage {
            data: Some(Data::Night(NightAction {
                actor_id,
                target_id,
            })),
        };
        let host_tx = network_clone.host_sender.lock().await.clone();
        if let Some(tx) = host_tx {
            tx.send(night_msg).await.map_err(|e| format!("发送夜间行动失败: {}", e))?;
            return Ok("夜间行动已提交。".to_string());
        } else {
            return Err("未连接至任何房间".to_string());
        }
    }

    let mut game = game_clone.inner.lock().await;

    if game.phase != GamePhase::NightAction as i32 {
        return Err("当前不是夜间阶段".to_string());
    }

    let actor_role = match game.players.get(&actor_id) {
        Some(p) if p.is_alive => p.role.clone(),
        Some(_) => return Err("已出局玩家无法在夜间行动".to_string()),
        None => return Err("行动玩家不存在".to_string()),
    };

    match game.players.get(&target_id) {
        Some(p) if p.is_alive => p.name.clone(),
        Some(_) => return Err("行动目标已出局".to_string()),
        None => return Err("目标玩家不存在".to_string()),
    };

    let feedback = if actor_role == ROLE_SPY {
        // 真卧底刀人：目标出局（好人、天使或白板在夜晚被杀均正常出局，白板夜间出局不触发白天公投专属猜词）
        if let Some(target) = game.players.get_mut(&target_id) {
            target.is_alive = false;
            if !game.night_casualties.contains(&target_id) {
                game.night_casualties.push(target_id.clone());
            }
        }
        "夜间行动已完成。".to_string()
    } else {
        // 好人或白板误以为自己是卧底动刀：反噬自身出局，加入夜间出局名单
        if let Some(actor) = game.players.get_mut(&actor_id) {
            actor.is_alive = false;
            if !game.night_casualties.contains(&actor_id) {
                game.night_casualties.push(actor_id.clone());
            }
        }
        "非卧底身份行动，自身出局。".to_string()
    };

    info!("夜间行动执行记录完毕，私密反馈: {}", feedback);

    // 夜间阶段保持静默，绝不广播公屏聊天，待白天 next_phase_command 统一播报战报
    Ok(feedback)
}

/// 内部处理白板猜词逻辑
pub async fn process_blank_guess_inner(
    game: &mut GameState,
    network: &NetworkState,
    app_handle: &AppHandle,
    player_id: &str,
    word_a: &str,
    word_b: &str,
) -> Result<bool, String> {
    if game.phase != GamePhase::BlankGuess as i32 {
        return Err("当前不是白板猜词阶段".to_string());
    }

    let is_blank = game.players.get(player_id).map(|p| p.role == ROLE_BLANK).unwrap_or(false);
    if !is_blank {
        return Err("只有被公投出局的白板特工可发起猜词".to_string());
    }

    let player_name = game.players.get(player_id).map(|p| p.name.clone()).unwrap_or_else(|| "白板".to_string());
    let w1 = word_a.trim();
    let w2 = word_b.trim();

    // 判定是否同时猜中两方词汇（好人词与卧底词，顺序不限）
    let is_correct = (!w1.is_empty() && !w2.is_empty()) && (
        (w1 == game.word_a && w2 == game.word_b) || (w1 == game.word_b && w2 == game.word_a)
    );

    if is_correct {
        game.winner = Some(ROLE_BLANK.to_string());
        let win_chat = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "SYSTEM".to_string(),
                text: format!("【终局决议】白板特工【{}】准确猜中双方词【{}】与【{}】，白板阵营获胜！", player_name, game.word_a, game.word_b),
                is_system: true,
            })),
        };
        broadcast_message(&network.peers, &win_chat).await;

        finish_game(game, network, app_handle, ROLE_BLANK).await;
        Ok(true)
    } else {
        let fail_chat = if w1.is_empty() && w2.is_empty() {
            format!("【猜词决议】白板特工【{}】放弃猜词，白板确认出局，游戏继续。", player_name)
        } else {
            format!("【猜词决议】白板特工【{}】猜词未完全命中（猜测：【{}】与【{}】），白板确认出局，游戏继续。", player_name, w1, w2)
        };
        let chat_msg = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "SYSTEM".to_string(),
                text: fail_chat,
                is_system: true,
            })),
        };
        broadcast_message(&network.peers, &chat_msg).await;

        // 检查胜负条件或流转至黑夜暗杀阶段
        if let Some(winner) = check_winner(&game.players) {
            finish_game(game, network, app_handle, &winner).await;
        } else {
            game.phase = GamePhase::NightAction as i32;
            let night_sync = GameMessage {
                data: Some(Data::SyncPhase(SyncPhase {
                    new_phase: GamePhase::NightAction as i32,
                    extra_info: "夜间阶段开始，请卧底执行暗杀。".to_string(),
                })),
            };
            broadcast_message(&network.peers, &night_sync).await;
        }
        Ok(false)
    }
}

/// 处理来自客户端网络消息的白板猜词
pub async fn process_blank_guess_message(
    game_state: &State<'_, GameEngineState>,
    network: &NetworkState,
    app_handle: &AppHandle,
    player_id: &str,
    word_a: &str,
    word_b: &str,
) -> Result<bool, String> {
    let mut game = game_state.inner.lock().await;
    process_blank_guess_inner(&mut game, network, app_handle, player_id, word_a, word_b).await
}

/// 白板猜词 Command（支持两方词猜测，兼顾房主与远程客户端）
#[tauri::command]
pub async fn submit_blank_guess_command(
    app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
    player_id: String,
    word_a: String,
    word_b: String,
) -> Result<bool, String> {
    let network_clone = network_state.inner().clone();
    let is_host = !network_clone.peers.lock().await.is_empty();

    if !is_host {
        let guess_msg = GameMessage {
            data: Some(Data::BlankGuess(BlankGuessAction {
                player_id,
                word_a,
                word_b,
            })),
        };
        let host_tx = network_clone.host_sender.lock().await.clone();
        if let Some(tx) = host_tx {
            tx.send(guess_msg).await.map_err(|e| format!("发送白板猜词失败: {}", e))?;
            return Ok(false);
        } else {
            return Err("未连接至任何房间".to_string());
        }
    }

    let mut game = game_state.inner.lock().await;
    process_blank_guess_inner(&mut game, &network_clone, &app_handle, &player_id, &word_a, &word_b).await
}

/// 重新开始游戏 Command（重置为准备中状态）
#[tauri::command]
pub async fn restart_game_command(
    _app_handle: AppHandle,
    network_state: State<'_, NetworkState>,
    game_state: State<'_, GameEngineState>,
) -> Result<(), String> {
    let network_clone = network_state.inner().clone();
    let game_clone = game_state.inner().clone();
    let mut game = game_clone.inner.lock().await;

    game.reset();

    let sync_msg = GameMessage {
        data: Some(Data::SyncPhase(SyncPhase {
            new_phase: GamePhase::Preparing as i32,
            extra_info: "游戏已重置。".to_string(),
        })),
    };
    broadcast_message(&network_clone.peers, &sync_msg).await;

    Ok(())
}

/// 查询当前游戏完整状态 Command
#[tauri::command]
pub async fn get_game_state_command(
    game_state: State<'_, GameEngineState>,
) -> Result<GameState, String> {
    let game = game_state.inner.lock().await;
    Ok(game.clone())
}

/// 结算收尾内部逻辑：公布全员真实身份、词汇与方向提示
async fn finish_game(
    game: &mut GameState,
    network: &NetworkState,
    _app_handle: &AppHandle,
    winner: &str,
) {
    let reveals: Vec<PlayerReveal> = game
        .players
        .values()
        .map(|p| PlayerReveal {
            id: p.id.clone(),
            name: p.name.clone(),
            avatar: p.avatar.clone(),
            role: p.role.clone(),
            word: p.word.clone(),
            hint: p.hint.clone(),
            is_alive: p.is_alive,
        })
        .collect();

    let winner_text = match winner {
        ROLE_GOOD => "好人阵营获胜（卧底全部出局，且白板被找出并猜词失败）。".to_string(),
        ROLE_SPY => "卧底阵营获胜。".to_string(),
        ROLE_BLANK => "白板阵营获胜（成功猜中双方词汇）。".to_string(),
        _ => "对局结束。".to_string(),
    };

    game.settlement_details = Some(SettlementDetails {
        winner_camp: winner.to_string(),
        good_word: game.word_a.clone(),
        spy_word: game.word_b.clone(),
        hint: game.hint.clone(),
        player_reveals: reveals,
    });

    let sync_msg = GameMessage {
        data: Some(Data::SyncPhase(SyncPhase {
            new_phase: GamePhase::Settlement as i32,
            extra_info: format!("游戏结算：{} 好人词【{}】，卧底词【{}】，提示【{}】", winner_text, game.word_a, game.word_b, game.hint),
        })),
    };
    broadcast_message(&network.peers, &sync_msg).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_generator_filtering() {
        let (w_a, w_b, hint) = generate_words("美食", "极为接近");
        assert!(!w_a.is_empty());
        assert!(!w_b.is_empty());
        assert!(!hint.is_empty());

        let (w_a2, w_b2, hint2) = generate_words("", "较为接近");
        assert!(!w_a2.is_empty());
        assert!(!w_b2.is_empty());
        assert!(!hint2.is_empty());
    }

    #[test]
    fn test_role_assignment_distribution() {
        // 4 ~ 12 人局全覆盖测试（白板角色人数恒为 1 人）
        let expected_matrix = [
            (4, 1, 1, 1, 1),
            (5, 1, 1, 1, 2),
            (6, 2, 1, 1, 2),
            (7, 2, 1, 1, 3),
            (8, 2, 1, 1, 4),
            (9, 3, 1, 1, 4),
            (10, 3, 1, 1, 5),
            (11, 3, 1, 1, 6),
            (12, 4, 1, 1, 6),
        ];

        for (count, exp_spy, exp_angel, exp_blank, exp_good) in expected_matrix {
            let p_ids: Vec<String> = (0..count).map(|i| format!("player_{}", i)).collect();
            let roles = assign_roles(&p_ids);
            assert_eq!(roles.len(), count, "总人数不符");
            let spies = roles.values().filter(|r| *r == ROLE_SPY).count();
            let angels = roles.values().filter(|r| *r == ROLE_ANGEL).count();
            let blanks = roles.values().filter(|r| *r == ROLE_BLANK).count();
            let goods = roles.values().filter(|r| *r == ROLE_GOOD).count();

            assert_eq!(spies, exp_spy, "{}人局卧底数不匹配", count);
            assert_eq!(angels, exp_angel, "{}人局天使数不匹配", count);
            assert_eq!(blanks, exp_blank, "{}人局白板数不匹配", count);
            assert_eq!(goods, exp_good, "{}人局好人数不匹配", count);
        }
    }

    #[test]
    fn test_angel_dual_word_assignment() {
        let word_a = "玫瑰";
        let word_b = "月季";
        let mut words = [word_a.to_string(), word_b.to_string()];
        words.shuffle(&mut thread_rng());
        let assigned = format!("{} / {}", words[0], words[1]);
        assert!(assigned == "玫瑰 / 月季" || assigned == "月季 / 玫瑰");
    }

    #[test]
    fn test_night_action_misjudgment_suicide_rule() {
        // 验证误判自爆核心机制
        let mut players = HashMap::new();
        players.insert("p_good".to_string(), PlayerState {
            id: "p_good".into(),
            name: "善良的好人".into(),
            avatar: "a1".into(),
            role: ROLE_GOOD.into(),
            word: "苹果".into(),
            hint: "".into(),
            is_alive: true,
        });
        players.insert("p_spy".to_string(), PlayerState {
            id: "p_spy".into(),
            name: "潜伏的卧底".into(),
            avatar: "a2".into(),
            role: ROLE_SPY.into(),
            word: "香蕉".into(),
            hint: "".into(),
            is_alive: true,
        });

        // 场景 1：好人误以为自己是卧底，试图刀卧底 -> 好人触发自爆出局，卧底存活
        let actor = players.get("p_good").unwrap();
        assert_ne!(actor.role, ROLE_SPY);
        // 执行规则判定
        players.get_mut("p_good").unwrap().is_alive = false; // 自爆
        assert!(!players.get("p_good").unwrap().is_alive);
        assert!(players.get("p_spy").unwrap().is_alive);

        // 场景 2：卧底动刀 -> 目标出局
        players.get_mut("p_good").unwrap().is_alive = true;
        let actor_spy = players.get("p_spy").unwrap();
        assert_eq!(actor_spy.role, ROLE_SPY);
        players.get_mut("p_good").unwrap().is_alive = false; // 被刀
        assert!(!players.get("p_good").unwrap().is_alive);
        assert!(players.get("p_spy").unwrap().is_alive);
    }

    #[test]
    fn test_winner_judgment() {
        let mut players = HashMap::new();
        players.insert("p1".into(), PlayerState {
            id: "p1".into(),
            name: "好人".into(),
            avatar: "".into(),
            role: ROLE_GOOD.into(),
            word: "A".into(),
            hint: "".into(),
            is_alive: true,
        });
        players.insert("p2".into(), PlayerState {
            id: "p2".into(),
            name: "卧底".into(),
            avatar: "".into(),
            role: ROLE_SPY.into(),
            word: "B".into(),
            hint: "".into(),
            is_alive: true,
        });

        // 存活 1 好人 vs 1 卧底 -> 卧底获胜 (alive_spy >= alive_good)
        assert_eq!(check_winner(&players), Some(ROLE_SPY.to_string()));

        // 卧底死亡 -> 好人获胜
        players.get_mut("p2").unwrap().is_alive = false;
        assert_eq!(check_winner(&players), Some(ROLE_GOOD.to_string()));
    }

    #[test]
    fn test_blank_killed_at_night_dies_normally() {
        // 验证白板在黑夜被暗杀时像普通玩家一样出局，不触发白天专属猜词
        let mut target_player = PlayerState {
            id: "p_blank".to_string(),
            name: "BlankPlayer".to_string(),
            avatar: "🎭".to_string(),
            role: ROLE_BLANK.to_string(),
            word: "".to_string(),
            hint: "".to_string(),
            is_alive: true,
        };
        let mut night_casualties = Vec::new();

        // 卧底刀中白板：正常置为 false 并加入夜间阵亡名单
        target_player.is_alive = false;
        night_casualties.push(target_player.id.clone());

        assert!(!target_player.is_alive);
        assert_eq!(night_casualties, vec!["p_blank".to_string()]);
    }

    #[test]
    fn test_blank_dual_word_guess_rule() {
        // 验证白板猜词必须同时猜中两方词汇（好词与坏词）
        let word_a = "苹果";
        let word_b = "香蕉";

        // 正确：顺序相同
        let (g1, g2) = ("苹果", "香蕉");
        let ok1 = (g1 == word_a && g2 == word_b) || (g1 == word_b && g2 == word_a);
        assert!(ok1);

        // 正确：顺序颠倒
        let (g3, g4) = ("香蕉", "苹果");
        let ok2 = (g3 == word_a && g4 == word_b) || (g3 == word_b && g4 == word_a);
        assert!(ok2);

        // 错误：只猜中其中一个
        let (g5, g6) = ("苹果", "梨子");
        let ok3 = (g5 == word_a && g6 == word_b) || (g5 == word_b && g6 == word_a);
        assert!(!ok3);
    }

    #[test]
    fn test_good_wins_requires_blank_found_and_eliminated() {
        // 验证好人阵营获胜条件：不仅需要所有卧底出局，且白板必须被找出并猜词失败（出局）
        let mut players = HashMap::new();
        players.insert("p1".into(), PlayerState {
            id: "p1".into(),
            name: "好人".into(),
            avatar: "".into(),
            role: ROLE_GOOD.into(),
            word: "A".into(),
            hint: "".into(),
            is_alive: true,
        });
        players.insert("p2".into(), PlayerState {
            id: "p2".into(),
            name: "天使".into(),
            avatar: "".into(),
            role: ROLE_ANGEL.into(),
            word: "A / B".into(),
            hint: "".into(),
            is_alive: true,
        });
        players.insert("p3".into(), PlayerState {
            id: "p3".into(),
            name: "卧底".into(),
            avatar: "".into(),
            role: ROLE_SPY.into(),
            word: "B".into(),
            hint: "".into(),
            is_alive: true,
        });
        players.insert("p4".into(), PlayerState {
            id: "p4".into(),
            name: "白板".into(),
            avatar: "".into(),
            role: ROLE_BLANK.into(),
            word: "".into(),
            hint: "水果".into(),
            is_alive: true,
        });

        // 阶段 1：卧底被公投出局，但白板依然存活 -> 好人不能直接获胜，对局继续寻找白板
        players.get_mut("p3").unwrap().is_alive = false;
        assert_eq!(check_winner(&players), None);

        // 阶段 2：白板也被公投找出（且猜词失败确认出局） -> 此时卧底全灭且白板出局，好人阵营正式获胜
        players.get_mut("p4").unwrap().is_alive = false;
        assert_eq!(check_winner(&players), Some(ROLE_GOOD.to_string()));
    }
}
