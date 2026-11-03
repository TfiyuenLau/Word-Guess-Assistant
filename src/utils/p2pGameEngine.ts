import {
  GamePhase,
  Player,
  FullGameState,
  SettlementDetails,
  PlayerReveal,
  IdentityAssignment,
} from "../types/game";

export const ROLE_GOOD = "GOOD";
export const ROLE_SPY = "SPY";
export const ROLE_ANGEL = "ANGEL";
export const ROLE_BLANK = "BLANK";

export interface WordPairItem {
  wordA: string;
  wordB: string;
  hint: string;
  similarity: string;
  theme: string;
}

export const BUILTIN_WORD_PAIRS: WordPairItem[] = [
  { wordA: "状元", wordB: "榜眼", hint: "古代科举及第名次", similarity: "极为接近", theme: "历史文化" },
  { wordA: "面包", wordB: "蛋糕", hint: "烘焙面食甜点", similarity: "极为接近", theme: "食物与饮品" },
  { wordA: "牛奶", wordB: "豆浆", hint: "常见早餐饮品", similarity: "极为接近", theme: "食物与饮品" },
  { wordA: "微信", wordB: "QQ", hint: "主流即时通讯社交软件", similarity: "极为接近", theme: "科技与软件" },
  { wordA: "西瓜", wordB: "哈密瓜", hint: "夏季常见多汁瓜果", similarity: "极为接近", theme: "食物与饮品" },
  { wordA: "自行车", wordB: "电动车", hint: "短途两轮代步工具", similarity: "极为接近", theme: "日常出行" },
  { wordA: "眉毛", wordB: "胡须", hint: "面部长出的毛发", similarity: "极为接近", theme: "人体生理" },
  { wordA: "玫瑰", wordB: "月季", hint: "常见观赏性刺木多花植物", similarity: "极为接近", theme: "自然花卉" },
  { wordA: "蝴蝶", wordB: "蜜蜂", hint: "花丛中穿梭采蜜授粉的昆虫", similarity: "极为接近", theme: "自然动物" },
  { wordA: "雨伞", wordB: "雨衣", hint: "雨天出门防雨用具", similarity: "极为接近", theme: "日常用品" },
  { wordA: "警察", wordB: "保安", hint: "维护治安秩序的人员", similarity: "极为接近", theme: "社会职业" },
  { wordA: "钢笔", wordB: "圆珠笔", hint: "日常纸质书写工具", similarity: "极为接近", theme: "文具学习" },
  { wordA: "魔术师", wordB: "特异功能", hint: "展示不可思议障眼现象", similarity: "高度相似", theme: "文娱表演" },
  { wordA: "牛肉干", wordB: "猪肉脯", hint: "便携耐嚼肉类零食", similarity: "高度相似", theme: "食物与饮品" },
  { wordA: "洗发水", wordB: "沐浴露", hint: "个人卫浴清洁洗护液体", similarity: "高度相似", theme: "生活卫浴" },
  { wordA: "高铁", wordB: "动车", hint: "现代轨道高速列车", similarity: "极为接近", theme: "日常出行" },
  { wordA: "咖啡", wordB: "奶茶", hint: "现代年轻人日常提神饮品", similarity: "高度相似", theme: "食物与饮品" },
  { wordA: "毛巾", wordB: "浴巾", hint: "擦拭水分的织物布品", similarity: "极为接近", theme: "日常用品" },
];

export interface P2PEnginePlayer extends Player {
  role?: string;
  word?: string;
  hint?: string;
  isAlive: boolean;
}

export class P2PGameEngine {
  public phase: GamePhase = GamePhase.PREPARING;
  public round = 1;
  public wordA = "";
  public wordB = "";
  public hint = "";
  public theme = "";
  public similarityLevel = "极为接近";
  public players: Map<string, P2PEnginePlayer> = new Map();
  public votes: Map<string, string> = new Map(); // voterId -> targetId
  public nightActions: Map<string, string> = new Map(); // actorId -> targetId
  public winner: string | null = null;
  public settlementDetails: SettlementDetails | null = null;
  public nightCasualties: string[] = [];
  public lastEliminatedPlayerId: string | null = null;

  constructor() {}

  public reset() {
    this.phase = GamePhase.PREPARING;
    this.round = 1;
    this.wordA = "";
    this.wordB = "";
    this.hint = "";
    this.theme = "";
    this.votes.clear();
    this.nightActions.clear();
    this.winner = null;
    this.settlementDetails = null;
    this.nightCasualties = [];
    this.lastEliminatedPlayerId = null;

    // 保留玩家列表但重置状态
    this.players.forEach((p) => {
      p.isAlive = true;
      p.role = undefined;
      p.word = undefined;
      p.hint = undefined;
    });
  }

  public addPlayer(player: Player) {
    this.players.set(player.id, {
      ...player,
      isAlive: true,
    });
  }

  public removePlayer(playerId: string) {
    this.players.delete(playerId);
    this.votes.delete(playerId);
    this.nightActions.delete(playerId);
  }

  /**
   * 启动游戏并执行角色词汇分配
   */
  public startGame(theme?: string, similarity?: string): Map<string, IdentityAssignment> {
    const list = Array.from(this.players.values());
    const count = list.length;
    if (count < 4) {
      throw new Error(`人数不足，至少需要 4 名特工方可启动博弈（当前 ${count} 人）`);
    }

    this.reset();
    this.similarityLevel = similarity || "极为接近";

    // 随机选词
    let candidates = BUILTIN_WORD_PAIRS;
    if (theme) {
      const filtered = candidates.filter((p) => p.theme === theme);
      if (filtered.length > 0) candidates = filtered;
    }
    const chosen = candidates[Math.floor(Math.random() * candidates.length)];
    this.wordA = chosen.wordA;
    this.wordB = chosen.wordB;
    this.hint = chosen.hint;
    this.theme = chosen.theme;

    // 动态身份配比 (4~12 人：固定 1 天使，固定 1 白板，其余按比例分配卧底与好人)
    let spyCount = 1;
    if (count >= 12) spyCount = 4;
    else if (count >= 9) spyCount = 3;
    else if (count >= 6) spyCount = 2;

    const roles: string[] = [];
    roles.push(ROLE_ANGEL);
    roles.push(ROLE_BLANK);
    for (let i = 0; i < spyCount; i++) roles.push(ROLE_SPY);
    const goodCount = count - roles.length;
    for (let i = 0; i < goodCount; i++) roles.push(ROLE_GOOD);

    // 乱序分配
    for (let i = roles.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [roles[i], roles[j]] = [roles[j], roles[i]];
    }

    const assignments = new Map<string, IdentityAssignment>();

    list.forEach((p, idx) => {
      const role = roles[idx];
      const enginePlayer = this.players.get(p.id)!;
      enginePlayer.role = role;
      enginePlayer.isAlive = true;

      if (role === ROLE_GOOD) {
        enginePlayer.word = this.wordA;
        assignments.set(p.id, {
          role: "AGENT", // 发词阶段盲盒，不显示 GOOD
          word: this.wordA,
          hint: "",
        });
      } else if (role === ROLE_SPY) {
        enginePlayer.word = this.wordB;
        assignments.set(p.id, {
          role: "AGENT", // 发词阶段盲盒，不显示 SPY
          word: this.wordB,
          hint: "",
        });
      } else if (role === ROLE_ANGEL) {
        // 天使获知好坏两词，顺序随机
        const words = Math.random() > 0.5 ? [this.wordA, this.wordB] : [this.wordB, this.wordA];
        enginePlayer.word = words.join(" / ");
        assignments.set(p.id, {
          role: ROLE_ANGEL,
          word: words.join(" / "),
          hint: "您同时感知到两方词语，请在不暴露自身的前提下引导平民！",
        });
      } else if (role === ROLE_BLANK) {
        enginePlayer.word = "";
        enginePlayer.hint = this.hint;
        assignments.set(p.id, {
          role: ROLE_BLANK,
          word: "【无词】",
          hint: this.hint,
        });
      }
    });

    this.phase = GamePhase.DESCRIPTION;
    return assignments;
  }

  /**
   * 提交投票
   */
  public submitVote(voterId: string, targetId: string) {
    const voter = this.players.get(voterId);
    if (!voter || !voter.isAlive) return;
    this.votes.set(voterId, targetId);
  }

  /**
   * 推进至下一阶段
   */
  public advancePhase(): { phase: GamePhase; eliminatedId?: string; announcement: string } {
    if (this.phase === GamePhase.DESCRIPTION) {
      this.phase = GamePhase.VOTING;
      this.votes.clear();
      return {
        phase: GamePhase.VOTING,
        announcement: "白天发言完毕，进入表决投票阶段！请各位特工慎重投票。",
      };
    } else if (this.phase === GamePhase.VOTING) {
      // 计票
      const tally = new Map<string, number>();
      this.votes.forEach((targetId) => {
        tally.set(targetId, (tally.get(targetId) || 0) + 1);
      });

      let maxVotes = 0;
      let eliminatedId: string | undefined = undefined;
      let isTie = false;

      tally.forEach((count, targetId) => {
        if (count > maxVotes) {
          maxVotes = count;
          eliminatedId = targetId;
          isTie = false;
        } else if (count === maxVotes) {
          isTie = true;
        }
      });

      if (isTie || !eliminatedId || maxVotes === 0) {
        this.phase = GamePhase.NIGHT_ACTION;
        return {
          phase: GamePhase.NIGHT_ACTION,
          announcement: "公投平票或无人得票，无人出局。天黑请闭眼，黑夜降临...",
        };
      }

      const eliminated = this.players.get(eliminatedId);
      if (eliminated) {
        eliminated.isAlive = false;
      }
      this.lastEliminatedPlayerId = eliminatedId;

      // 核心规则：若白板在白天被公投出局，立即触发白板专属猜词特别阶段！
      if (eliminated && eliminated.role === ROLE_BLANK) {
        this.phase = GamePhase.BLANK_GUESS;
        return {
          phase: GamePhase.BLANK_GUESS,
          eliminatedId,
          announcement: `特工【${eliminated.name}】被公投出局！其真实身份为白板，触发白板终局双词逆袭！`,
        };
      }

      // 检查胜负
      if (this.checkWinCondition()) {
        this.phase = GamePhase.SETTLEMENT;
        return {
          phase: GamePhase.SETTLEMENT,
          eliminatedId,
          announcement: `特工【${eliminated?.name}】出局。对局已决出胜负！`,
        };
      }

      this.phase = GamePhase.NIGHT_ACTION;
      return {
        phase: GamePhase.NIGHT_ACTION,
        eliminatedId,
        announcement: `特工【${eliminated?.name}】在公投中以 ${maxVotes} 票出局。天黑请闭眼...`,
      };
    } else if (this.phase === GamePhase.BLANK_GUESS) {
      // 若白板放弃猜词或未猜对，如常推进至夜间
      if (this.checkWinCondition()) {
        this.phase = GamePhase.SETTLEMENT;
        return {
          phase: GamePhase.SETTLEMENT,
          announcement: "对局结束，进入战报结算！",
        };
      }
      this.phase = GamePhase.NIGHT_ACTION;
      return {
        phase: GamePhase.NIGHT_ACTION,
        announcement: "白板逆袭结束。天黑请闭眼，夜幕降临...",
      };
    } else if (this.phase === GamePhase.NIGHT_ACTION) {
      // 结算黑夜刺杀与误动自爆
      this.nightCasualties = [];

      this.nightActions.forEach((targetId, actorId) => {
        const actor = this.players.get(actorId);
        if (!actor || !actor.isAlive) return;

        if (actor.role === ROLE_SPY) {
          if (targetId && targetId !== actorId) {
            const target = this.players.get(targetId);
            if (target && target.isAlive) {
              target.isAlive = false;
              if (!this.nightCasualties.includes(targetId)) {
                this.nightCasualties.push(targetId);
              }
            }
          }
        } else {
          // 好人、天使或白板夜间擅自动刀，属于误判自爆，自身意外出局
          actor.isAlive = false;
          if (!this.nightCasualties.includes(actorId)) {
            this.nightCasualties.push(actorId);
          }
        }
      });

      this.nightActions.clear();

      // 胜负判定
      if (this.checkWinCondition()) {
        this.phase = GamePhase.SETTLEMENT;
        return {
          phase: GamePhase.SETTLEMENT,
          announcement: "黑夜行动结束，对局已决出最终胜负！",
        };
      }

      this.phase = GamePhase.DESCRIPTION;
      this.round += 1;

      let announceText = "天亮了！";
      if (this.nightCasualties.length === 0) {
        announceText += " 昨夜是平安夜，无人伤亡。";
      } else {
        const names = this.nightCasualties
          .map((id) => `【${this.players.get(id)?.name}】`)
          .join("、");
        announceText += ` 昨夜 ${names} 倒在暗影中，不幸出局。`;
      }

      return {
        phase: GamePhase.DESCRIPTION,
        announcement: announceText,
      };
    }

    return {
      phase: this.phase,
      announcement: "",
    };
  }

  /**
   * 提交黑夜行动
   */
  public submitNightAction(actorId: string, targetId: string) {
    const actor = this.players.get(actorId);
    if (!actor || !actor.isAlive) return;
    this.nightActions.set(actorId, targetId);
  }

  /**
   * 白板提交双词竞猜
   */
  public submitBlankGuess(playerId: string, word1: string, word2: string): boolean {
    const player = this.players.get(playerId);
    if (!player || player.role !== ROLE_BLANK) return false;

    const w1 = word1.trim();
    const w2 = word2.trim();

    // 需完全命中好人词与卧底词两方（无序对比）
    const isSuccess =
      (w1 === this.wordA && w2 === this.wordB) || (w1 === this.wordB && w2 === this.wordA);

    if (isSuccess) {
      this.winner = "BLANK";
      this.buildSettlement("白板独胜 (双词准确逆袭)");
      this.phase = GamePhase.SETTLEMENT;
      return true;
    }
    return false;
  }

  /**
   * 判定对局胜负
   */
  public checkWinCondition(): boolean {
    if (this.winner) return true;

    let aliveSpies = 0;
    let aliveGood = 0; // Good + Angel
    let blankFoundAndDead = false;

    this.players.forEach((p) => {
      if (p.isAlive) {
        if (p.role === ROLE_SPY) aliveSpies++;
        else if (p.role === ROLE_GOOD || p.role === ROLE_ANGEL) aliveGood++;
      } else {
        if (p.role === ROLE_BLANK) blankFoundAndDead = true;
      }
    });

    // 1. 卧底胜利：存活卧底人数 >= 存活好人人数
    if (aliveSpies >= aliveGood) {
      this.winner = "SPY";
      this.buildSettlement("卧底阵营获胜 (好人防线崩溃)");
      return true;
    }

    // 2. 好人胜利：卧底全部被投出出局，且白板被找出出局并猜词失败
    if (aliveSpies === 0 && blankFoundAndDead) {
      this.winner = "GOOD";
      this.buildSettlement("好人阵营获胜 (肃清全部卧底并排查白板)");
      return true;
    }

    return false;
  }

  /**
   * 构建最终结算战报
   */
  private buildSettlement(winnerCamp: string) {
    const reveals: PlayerReveal[] = Array.from(this.players.values()).map((p) => ({
      id: p.id,
      name: p.name,
      avatar: p.avatar,
      role: p.role || ROLE_GOOD,
      word: p.word || "",
      hint: p.hint || "",
      is_alive: p.isAlive,
    }));

    this.settlementDetails = {
      winner_camp: winnerCamp,
      good_word: this.wordA,
      spy_word: this.wordB,
      hint: this.hint,
      player_reveals: reveals,
    };
  }

  /**
   * 获取快照
   */
  public getSnapshot(): FullGameState {
    const playersObj: Record<string, Player> = {};
    this.players.forEach((p, id) => {
      playersObj[id] = {
        id: p.id,
        name: p.name,
        avatar: p.avatar,
        role: this.phase === GamePhase.SETTLEMENT ? p.role : undefined,
        word: this.phase === GamePhase.SETTLEMENT ? p.word : undefined,
        hint: this.phase === GamePhase.SETTLEMENT ? p.hint : undefined,
        is_alive: p.isAlive,
      };
    });

    const votesObj: Record<string, string> = {};
    this.votes.forEach((v, k) => {
      votesObj[k] = v;
    });

    return {
      phase: this.phase,
      word_a: this.phase === GamePhase.SETTLEMENT ? this.wordA : "",
      word_b: this.phase === GamePhase.SETTLEMENT ? this.wordB : "",
      hint: this.phase === GamePhase.SETTLEMENT ? this.hint : "",
      similarity_level: this.similarityLevel,
      theme: this.theme,
      round: this.round,
      winner: this.winner || undefined,
      settlement_details: this.settlementDetails || undefined,
      players: playersObj,
      votes: votesObj,
    };
  }
}
