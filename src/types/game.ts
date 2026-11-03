// 游戏阶段枚举（严格与 protobuf GamePhase 对应）
export enum GamePhase {
  PREPARING = 0,
  DESCRIPTION = 1,
  VOTING = 2,
  NIGHT_ACTION = 3,
  SETTLEMENT = 4,
  BLANK_GUESS = 5,
}

// 身份枚举
export const ROLE_GOOD = "GOOD";
export const ROLE_SPY = "SPY";
export const ROLE_ANGEL = "ANGEL";
export const ROLE_BLANK = "BLANK";
export const ROLE_SPECTATOR = "SPECTATOR";

export interface JoinRequest {
  player_id: string;
  player_name: string;
  avatar: string;
}

export interface LeaveRequest {
  player_id: string;
}

export interface IdentityAssignment {
  role: string;
  word: string;
  hint: string;
}

export interface ChatMessage {
  sender_id: string;
  text: string;
  is_system: boolean;
  timestamp?: number;
}

export interface VoteAction {
  voter_id: string;
  target_id: string;
}

export interface NightAction {
  actor_id: string;
  target_id: string;
}

export interface BlankGuessAction {
  player_id: string;
  word_a: string;
  word_b: string;
}

export interface SyncPhase {
  new_phase: number;
  extra_info: string;
}

export interface Player {
  id: string;
  name: string;
  avatar: string;
  role?: string;
  word?: string;
  hint?: string;
  is_alive: boolean;
}

export interface PlayerReveal {
  id: string;
  name: string;
  avatar: string;
  role: string;
  word: string;
  hint: string;
  is_alive: boolean;
}

export interface SettlementDetails {
  winner_camp: string;
  good_word: string;
  spy_word: string;
  hint: string;
  player_reveals: PlayerReveal[];
}

export interface FullGameState {
  phase: number;
  word_a: string;
  word_b: string;
  hint: string;
  similarity_level: string;
  theme: string;
  players: Record<string, Player>;
  votes: Record<string, string>;
  round: number;
  winner?: string;
  settlement_details?: SettlementDetails;
}

export interface DiscoveredRoom {
  room_id: string;
  host_name: string;
  host_avatar: string;
  ip: string;
  port: number;
  player_count: number;
  max_players: number;
  phase: number;
  phase_title: string;
}


