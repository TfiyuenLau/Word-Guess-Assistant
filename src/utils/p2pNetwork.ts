import {
  DEFAULT_STUN_SERVERS,
  P2PSignaling,
  SignalingAnswerPayload,
  SignalingIcePayload,
  SignalingJoinPayload,
  SignalingOfferPayload,
} from "./p2pSignaling";
import { Player } from "../types/game";

export interface P2PMessage {
  type: string;
  senderId: string;
  payload: any;
}

export type P2PMessageHandler = (msg: P2PMessage, fromPeerId: string) => void;
export type P2PPeerEventHandler = (player: Player) => void;

export class P2PNetworkManager {
  private isHost = false;
  private selfPlayer: Player | null = null;
  private roomCode = "";
  private signaling: P2PSignaling | null = null;

  // 房主视角：维护所有客户端的连接 (peerId -> { pc, dc, player })
  private peerConnections: Map<
    string,
    { pc: RTCPeerConnection; dc?: RTCDataChannel; player?: Player }
  > = new Map();

  // 客户端视角：维护通往房主的连接
  private hostPc: RTCPeerConnection | null = null;
  private hostDc: RTCDataChannel | null = null;
  private hostPlayerId: string | null = null;

  private messageHandlers: P2PMessageHandler[] = [];
  private peerJoinHandlers: P2PPeerEventHandler[] = [];
  private peerLeaveHandlers: ((peerId: string) => void)[] = [];

  constructor() {}

  public getRoomCode(): string {
    return this.roomCode;
  }

  public getHostPlayerId(): string | null {
    return this.hostPlayerId;
  }

  /**
   * 房主启动 P2P 房间
   */
  public async createRoom(roomCode: string, hostPlayer: Player): Promise<void> {
    this.close();
    this.isHost = true;
    this.selfPlayer = hostPlayer;
    this.roomCode = roomCode;

    this.signaling = new P2PSignaling(roomCode, hostPlayer.id);

    await this.signaling.init(
      // onJoin
      async (joinData: SignalingJoinPayload) => {
        if (!this.isHost) return;
        await this.handleClientJoin(joinData);
      },
      undefined,
      // onAnswer
      async (answerData: SignalingAnswerPayload) => {
        if (!this.isHost) return;
        await this.handleClientAnswer(answerData);
      },
      // onIce
      async (iceData: SignalingIcePayload) => {
        if (!this.isHost) return;
        const peer = this.peerConnections.get(iceData.fromId);
        if (peer && peer.pc.remoteDescription) {
          try {
            await peer.pc.addIceCandidate(new RTCIceCandidate(iceData.candidate));
          } catch (e) {
            console.warn("房主添加 ICE 候选失败:", e);
          }
        }
      }
    );
  }

  /**
   * 普通玩家加入 P2P 房间
   */
  public async joinRoom(roomCode: string, clientPlayer: Player): Promise<void> {
    this.close();
    this.isHost = false;
    this.selfPlayer = clientPlayer;
    this.roomCode = roomCode;

    this.signaling = new P2PSignaling(roomCode, clientPlayer.id);

    await this.signaling.init(
      undefined,
      // onOffer
      async (offerData: SignalingOfferPayload) => {
        if (this.isHost) return;
        await this.handleHostOffer(offerData);
      },
      undefined,
      // onIce
      async (iceData: SignalingIcePayload) => {
        if (this.isHost) return;
        if (this.hostPc && this.hostPc.remoteDescription) {
          try {
            await this.hostPc.addIceCandidate(new RTCIceCandidate(iceData.candidate));
          } catch (e) {
            console.warn("客户端添加 ICE 候选失败:", e);
          }
        }
      }
    );

    // 发送加入申请
    this.signaling.sendJoin(clientPlayer.name, clientPlayer.avatar);
  }

  public onMessage(handler: P2PMessageHandler) {
    this.messageHandlers.push(handler);
  }

  public onPeerJoin(handler: P2PPeerEventHandler) {
    this.peerJoinHandlers.push(handler);
  }

  public onPeerLeave(handler: (peerId: string) => void) {
    this.peerLeaveHandlers.push(handler);
  }

  /**
   * 房主广播给所有已连接客户端
   */
  public broadcast(msg: P2PMessage): void {
    const raw = JSON.stringify(msg);
    this.peerConnections.forEach(({ dc }) => {
      if (dc && dc.readyState === "open") {
        dc.send(raw);
      }
    });
  }

  /**
   * 房主单播发送给指定客户端
   */
  public sendTo(peerId: string, msg: P2PMessage): void {
    const peer = this.peerConnections.get(peerId);
    if (peer && peer.dc && peer.dc.readyState === "open") {
      peer.dc.send(JSON.stringify(msg));
    }
  }

  /**
   * 客户端发送给房主
   */
  public sendToHost(msg: P2PMessage): void {
    if (this.hostDc && this.hostDc.readyState === "open") {
      this.hostDc.send(JSON.stringify(msg));
    }
  }

  /**
   * 关闭网络并清理资源
   */
  public close(): void {
    if (this.signaling) {
      this.signaling.close();
      this.signaling = null;
    }

    this.peerConnections.forEach(({ pc, dc }) => {
      try {
        dc?.close();
        pc.close();
      } catch (_) {}
    });
    this.peerConnections.clear();

    if (this.hostDc) {
      try {
        this.hostDc.close();
      } catch (_) {}
      this.hostDc = null;
    }

    if (this.hostPc) {
      try {
        this.hostPc.close();
      } catch (_) {}
      this.hostPc = null;
    }

    this.isHost = false;
    this.selfPlayer = null;
    this.roomCode = "";
  }

  // --- 内部信令与 WebRTC 握手逻辑 ---

  private async handleClientJoin(joinData: SignalingJoinPayload) {
    const clientId = joinData.clientId;
    if (this.peerConnections.has(clientId)) {
      try {
        this.peerConnections.get(clientId)?.pc.close();
      } catch (_) {}
    }

    const pc = new RTCPeerConnection({ iceServers: DEFAULT_STUN_SERVERS });
    const dc = pc.createDataChannel("wga-game", { ordered: true });

    const player: Player = {
      id: clientId,
      name: joinData.playerName,
      avatar: joinData.avatar,
      is_alive: true,
    };

    this.peerConnections.set(clientId, { pc, dc, player });

    this.setupDataChannel(dc, clientId);

    pc.onicecandidate = (event) => {
      if (event.candidate && this.signaling) {
        this.signaling.sendIce(clientId, event.candidate.toJSON());
      }
    };

    pc.onconnectionstatechange = () => {
      if (pc.connectionState === "disconnected" || pc.connectionState === "failed") {
        this.handlePeerDisconnected(clientId);
      }
    };

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    if (this.signaling) {
      this.signaling.sendOffer(clientId, offer);
    }
  }

  private async handleClientAnswer(answerData: SignalingAnswerPayload) {
    const peer = this.peerConnections.get(answerData.clientId);
    if (!peer) return;

    await peer.pc.setRemoteDescription(new RTCSessionDescription(answerData.sdp));
  }

  private async handleHostOffer(offerData: SignalingOfferPayload) {
    if (this.hostPc) {
      try {
        this.hostPc.close();
      } catch (_) {}
    }

    this.hostPc = new RTCPeerConnection({ iceServers: DEFAULT_STUN_SERVERS });

    this.hostPc.ondatachannel = (event) => {
      this.hostDc = event.channel;
      this.setupDataChannel(this.hostDc, "HOST");
    };

    this.hostPc.onicecandidate = (event) => {
      if (event.candidate && this.signaling) {
        // 向房主发送客户端 ICE candidate
        this.signaling.sendIce("HOST", event.candidate.toJSON());
      }
    };

    this.hostPc.onconnectionstatechange = () => {
      if (this.hostPc?.connectionState === "disconnected" || this.hostPc?.connectionState === "failed") {
        this.handlePeerDisconnected("HOST");
      }
    };

    await this.hostPc.setRemoteDescription(new RTCSessionDescription(offerData.sdp));
    const answer = await this.hostPc.createAnswer();
    await this.hostPc.setLocalDescription(answer);

    if (this.signaling && this.selfPlayer) {
      this.signaling.sendAnswer("HOST", answer);
    }
  }

  private setupDataChannel(dc: RTCDataChannel, remoteId: string) {
    dc.onopen = () => {
      console.info(`[WebRTC P2P] DataChannel 打开成功: ${remoteId}`);
      if (this.isHost) {
        const peer = this.peerConnections.get(remoteId);
        if (peer?.player) {
          for (const handler of this.peerJoinHandlers) {
            handler(peer.player);
          }
        }
      }
    };

    dc.onmessage = (event) => {
      try {
        const msg: P2PMessage = JSON.parse(event.data);
        for (const handler of this.messageHandlers) {
          handler(msg, remoteId);
        }
      } catch (e) {
        console.warn("解析 P2P 消息失败:", e);
      }
    };

    dc.onclose = () => {
      this.handlePeerDisconnected(remoteId);
    };
  }

  private handlePeerDisconnected(peerId: string) {
    if (this.isHost) {
      if (this.peerConnections.has(peerId)) {
        this.peerConnections.delete(peerId);
        for (const handler of this.peerLeaveHandlers) {
          handler(peerId);
        }
      }
    } else if (peerId === "HOST") {
      for (const handler of this.peerLeaveHandlers) {
        handler("HOST");
      }
    }
  }
}

export const globalP2PNetwork = new P2PNetworkManager();
