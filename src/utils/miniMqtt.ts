/**
 * MiniMqttClient - 极简零依赖 MQTT 3.1.1 over WebSocket 客户端
 * 专门用于 WebRTC P2P 瞬间信令握手，支持国内 EMQX / Mosquitto 等标准公共或私有 WSS 节点
 */

export interface MqttMessage {
  topic: string;
  payload: string;
}

export type MessageHandler = (msg: MqttMessage) => void;

export class MiniMqttClient {
  private ws: WebSocket | null = null;
  private url: string;
  private clientId: string;
  private messageHandlers: MessageHandler[] = [];
  private isConnected = false;
  private pingInterval: any = null;
  private packetId = 1;

  constructor(url: string, clientId?: string) {
    this.url = url;
    this.clientId = clientId || "wga_" + Math.random().toString(36).substring(2, 10);
  }

  public connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url, ["mqtt"]);
        this.ws.binaryType = "arraybuffer";

        const connectTimeout = setTimeout(() => {
          if (!this.isConnected) {
            this.disconnect();
            reject(new Error("信令服务器连接超时"));
          }
        }, 8000);

        this.ws.onopen = () => {
          this.sendConnect();
        };

        this.ws.onmessage = (event: MessageEvent) => {
          const data = new Uint8Array(event.data as ArrayBuffer);
          if (data.length === 0) return;

          const packetType = data[0] >> 4;

          // CONNACK (Packet Type 2)
          if (packetType === 2) {
            this.isConnected = true;
            clearTimeout(connectTimeout);
            this.startPing();
            resolve();
          }
          // PUBLISH (Packet Type 3)
          else if (packetType === 3) {
            this.handlePublish(data);
          }
        };

        this.ws.onerror = (err) => {
          clearTimeout(connectTimeout);
          reject(err);
        };

        this.ws.onclose = () => {
          this.isConnected = false;
          this.stopPing();
        };
      } catch (e) {
        reject(e);
      }
    });
  }

  public onMessage(handler: MessageHandler) {
    this.messageHandlers.push(handler);
  }

  public subscribe(topic: string): void {
    if (!this.ws || !this.isConnected) return;

    const topicBytes = new TextEncoder().encode(topic);
    const pid = this.nextPacketId();

    // Variable Header (2 bytes packetId) + Payload (topic length 2 bytes + topic + QoS 1 byte)
    const varHeaderAndPayload = new Uint8Array(2 + 2 + topicBytes.length + 1);
    varHeaderAndPayload[0] = (pid >> 8) & 0xff;
    varHeaderAndPayload[1] = pid & 0xff;
    varHeaderAndPayload[2] = (topicBytes.length >> 8) & 0xff;
    varHeaderAndPayload[3] = topicBytes.length & 0xff;
    varHeaderAndPayload.set(topicBytes, 4);
    varHeaderAndPayload[4 + topicBytes.length] = 0x00; // QoS 0

    const packet = this.buildPacket(0x82, varHeaderAndPayload);
    (this.ws as any).send(packet);
  }

  public publish(topic: string, message: string): void {
    if (!this.ws || !this.isConnected) return;

    const topicBytes = new TextEncoder().encode(topic);
    const payloadBytes = new TextEncoder().encode(message);

    // Variable Header (topic length 2 bytes + topic) + Payload
    const content = new Uint8Array(2 + topicBytes.length + payloadBytes.length);
    content[0] = (topicBytes.length >> 8) & 0xff;
    content[1] = topicBytes.length & 0xff;
    content.set(topicBytes, 2);
    content.set(payloadBytes, 2 + topicBytes.length);

    const packet = this.buildPacket(0x30, content);
    (this.ws as any).send(packet);
  }

  public disconnect(): void {
    this.stopPing();
    if (this.ws) {
      if (this.isConnected) {
        try {
          (this.ws as any).send(new Uint8Array([0xe0, 0x00])); // DISCONNECT
        } catch (_) {}
      }
      this.ws.close();
      this.ws = null;
    }
    this.isConnected = false;
  }

  private sendConnect(): void {
    if (!this.ws) return;

    const clientBytes = new TextEncoder().encode(this.clientId);
    const protoName = new TextEncoder().encode("MQTT");

    // Variable Header: "MQTT" (4 bytes) + Level (1 byte: 0x04) + Flags (1 byte: 0x02 CleanSession) + KeepAlive (2 bytes: 60)
    // Payload: Client ID (2 bytes length + client ID)
    const varAndPayloadLen = (2 + protoName.length + 1 + 1 + 2) + (2 + clientBytes.length);
    const content = new Uint8Array(varAndPayloadLen);

    let offset = 0;
    // Proto name
    content[offset++] = (protoName.length >> 8) & 0xff;
    content[offset++] = protoName.length & 0xff;
    content.set(protoName, offset);
    offset += protoName.length;

    content[offset++] = 0x04; // MQTT 3.1.1
    content[offset++] = 0x02; // Clean Session
    content[offset++] = 0x00; // Keep Alive MSB
    content[offset++] = 0x3c; // Keep Alive LSB (60s)

    // Client ID
    content[offset++] = (clientBytes.length >> 8) & 0xff;
    content[offset++] = clientBytes.length & 0xff;
    content.set(clientBytes, offset);

    const packet = this.buildPacket(0x10, content);
    (this.ws as any).send(packet);
  }

  private handlePublish(data: Uint8Array): void {
    try {
      // Decode remaining length
      let offset = 1;
      let multiplier = 1;
      let remainingLength = 0;
      let digit = 0;
      do {
        digit = data[offset++];
        remainingLength += (digit & 0x7f) * multiplier;
        multiplier *= 128;
      } while ((digit & 0x80) !== 0 && offset < data.length);

      // Topic length
      const topicLen = (data[offset] << 8) | data[offset + 1];
      offset += 2;
      const topic = new TextDecoder().decode(data.subarray(offset, offset + topicLen));
      offset += topicLen;

      // Payload
      const payload = new TextDecoder().decode(data.subarray(offset));
      for (const handler of this.messageHandlers) {
        handler({ topic, payload });
      }
    } catch (e) {
      console.warn("MQTT 消息解析异常:", e);
    }
  }

  private startPing(): void {
    this.stopPing();
    this.pingInterval = setInterval(() => {
      if (this.ws && this.isConnected) {
        try {
          this.ws.send(new Uint8Array([0xc0, 0x00])); // PINGREQ
        } catch (_) {}
      }
    }, 25000);
  }

  private stopPing(): void {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  private nextPacketId(): number {
    this.packetId = (this.packetId + 1) & 0xffff;
    return this.packetId === 0 ? 1 : this.packetId;
  }

  private buildPacket(header: number, content: Uint8Array): Uint8Array {
    let remLen = content.length;
    const lenBytes: number[] = [];
    do {
      let digit = remLen % 128;
      remLen = Math.floor(remLen / 128);
      if (remLen > 0) {
        digit |= 0x80;
      }
      lenBytes.push(digit);
    } while (remLen > 0);

    const packet = new Uint8Array(1 + lenBytes.length + content.length);
    packet[0] = header;
    packet.set(lenBytes, 1);
    packet.set(content, 1 + lenBytes.length);
    return packet;
  }
}
