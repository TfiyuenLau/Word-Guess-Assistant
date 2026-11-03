import { MiniMqttClient } from "./miniMqtt";

export const DEFAULT_STUN_SERVERS: RTCIceServer[] = [
  { urls: "stun:stun.qq.com:3478" },
  { urls: "stun:stun.miwifi.com:3478" },
  { urls: "stun:stun.chat.bilibili.com:3478" },
  { urls: "stun:stun.l.google.com:19302" },
];

export const DEFAULT_SIGNALING_URL = "wss://broker.emqx.io:8084/mqtt";

export function getCustomSignalingUrl(): string {
  return localStorage.getItem("wga_signaling_url") || DEFAULT_SIGNALING_URL;
}

export function setCustomSignalingUrl(url: string) {
  localStorage.setItem("wga_signaling_url", url);
}

export interface SignalingJoinPayload {
  clientId: string;
  playerName: string;
  avatar: string;
}

export interface SignalingOfferPayload {
  sdp: RTCSessionDescriptionInit;
  targetClientId: string;
}

export interface SignalingAnswerPayload {
  sdp: RTCSessionDescriptionInit;
  clientId: string;
}

export interface SignalingIcePayload {
  candidate: RTCIceCandidateInit;
  targetId: string;
  fromId: string;
}

export class P2PSignaling {
  private client: MiniMqttClient | null = null;
  private roomCode: string;
  private selfId: string;

  constructor(roomCode: string, selfId: string) {
    this.roomCode = roomCode;
    this.selfId = selfId;
  }

  public async init(
    onJoin?: (data: SignalingJoinPayload) => void,
    onOffer?: (data: SignalingOfferPayload) => void,
    onAnswer?: (data: SignalingAnswerPayload) => void,
    onIce?: (data: SignalingIcePayload) => void
  ): Promise<void> {
    const url = getCustomSignalingUrl();
    this.client = new MiniMqttClient(url, `${this.selfId}_${Date.now()}`);

    this.client.onMessage(({ topic, payload }) => {
      try {
        const data = JSON.parse(payload);
        const subTopic = topic.replace(`wga/room/${this.roomCode}/`, "");

        if (subTopic === "join" && onJoin) {
          onJoin(data);
        } else if (subTopic.startsWith("offer/") && onOffer) {
          const targetId = subTopic.replace("offer/", "");
          if (targetId === this.selfId) {
            onOffer(data);
          }
        } else if (subTopic.startsWith("answer/") && onAnswer) {
          const hostId = subTopic.replace("answer/", "");
          if (hostId === this.selfId) {
            onAnswer(data);
          }
        } else if (subTopic.startsWith("ice/") && onIce) {
          const targetId = subTopic.replace("ice/", "");
          if (targetId === this.selfId) {
            onIce(data);
          }
        }
      } catch (e) {
        console.warn("解析信令数据失败:", e);
      }
    });

    await this.client.connect();

    // 订阅当前房间相关主题
    this.client.subscribe(`wga/room/${this.roomCode}/#`);
  }

  public sendJoin(playerName: string, avatar: string) {
    if (!this.client) return;
    this.client.publish(
      `wga/room/${this.roomCode}/join`,
      JSON.stringify({ clientId: this.selfId, playerName, avatar })
    );
  }

  public sendOffer(targetClientId: string, sdp: RTCSessionDescriptionInit) {
    if (!this.client) return;
    this.client.publish(
      `wga/room/${this.roomCode}/offer/${targetClientId}`,
      JSON.stringify({ sdp, targetClientId })
    );
  }

  public sendAnswer(hostId: string, sdp: RTCSessionDescriptionInit) {
    if (!this.client) return;
    this.client.publish(
      `wga/room/${this.roomCode}/answer/${hostId}`,
      JSON.stringify({ sdp, clientId: this.selfId })
    );
  }

  public sendIce(targetId: string, candidate: RTCIceCandidateInit) {
    if (!this.client) return;
    this.client.publish(
      `wga/room/${this.roomCode}/ice/${targetId}`,
      JSON.stringify({ candidate, targetId, fromId: this.selfId })
    );
  }

  public close() {
    if (this.client) {
      this.client.disconnect();
      this.client = null;
    }
  }
}
