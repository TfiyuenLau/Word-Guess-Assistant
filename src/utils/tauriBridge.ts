import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { isTauri } from "./platform";

/**
 * 跨平台安全 invoke 包装器
 * 在 Tauri 原生环境下直通原生后端指令；在纯 Web 浏览器环境下提供兼容回退与友善提示
 */
export async function safeInvoke<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  if (isTauri()) {
    return await tauriInvoke<T>(cmd, args);
  }

  // 纯 Web 浏览器模式降级处理
  console.info(`[Web Mode Mock] safeInvoke called for command: ${cmd}`, args);

  switch (cmd) {
    case "discover_rooms_command":
      // Web 浏览器沙盒受同源与纯 HTTP/WS 限制，无法直接收发 UDP 组播探针
      return [] as unknown as T;

    case "create_game_room_command":
      console.info("[Web Mode] 浏览器模式：已模拟创建房间（网页端仅支持模拟或配合服务端网关）");
      return undefined as unknown as T;

    case "join_game_room_command":
      console.info("[Web Mode] 浏览器模式：已模拟加入房间", args);
      return undefined as unknown as T;

    case "close_game_room_command":
    case "leave_game_room_command":
    case "send_chat_command":
    case "start_game_command":
    case "next_phase_command":
    case "submit_vote_command":
    case "submit_night_action_command":
    case "submit_blank_guess_command":
    case "restart_game_command":
      return undefined as unknown as T;

    case "get_game_state_command":
      return null as unknown as T;

    case "get_room_players_command":
      return [] as unknown as T;

    default:
      console.warn(`[Web Mode] 未知原生指令回退: ${cmd}`);
      return undefined as unknown as T;
  }
}

