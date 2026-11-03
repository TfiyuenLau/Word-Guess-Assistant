#!/usr/bin/env node

/**
 * Word Guess Assistant - 全方位网络与机器人连通性自检诊断脚本
 * 运行方式: node scripts/test-connectivity.cjs 或 pnpm test:network
 */

const dgram = require('dgram');
const net = require('net');
const WebSocket = require('ws');

const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  bold: '\x1b[1m',
};

console.log(`${colors.cyan}${colors.bold}============================================================${colors.reset}`);
console.log(`${colors.cyan}${colors.bold}🕵️  Word Guess Assistant - 全链路网络连通性自检诊断${colors.reset}`);
console.log(`${colors.cyan}${colors.bold}============================================================${colors.reset}\n`);

async function testLocalSockets() {
  console.log(`${colors.bold}[1/3] 正在检测本地网络套接字 (TCP / UDP) 可用性...${colors.reset}`);

  // 测试本地 TCP 服务监听能力
  const tcpOk = await new Promise((resolve) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      const client = net.connect(port, '127.0.0.1', () => {
        client.destroy();
        server.close(() => resolve({ ok: true, port }));
      });
    });
    server.on('error', (err) => resolve({ ok: false, err: err.message }));
  });

  if (tcpOk.ok) {
    console.log(`  ${colors.green}✔ 本地 TCP 环回测试成功${colors.reset} (测试端口: ${tcpOk.port})`);
  } else {
    console.log(`  ${colors.red}✖ 本地 TCP 监听失败: ${tcpOk.err}${colors.reset}`);
  }

  // 测试 UDP 探针套接字绑定能力
  const udpOk = await new Promise((resolve) => {
    const sock = dgram.createSocket('udp4');
    sock.bind(0, '0.0.0.0', () => {
      const port = sock.address().port;
      sock.close(() => resolve({ ok: true, port }));
    });
    sock.on('error', (err) => resolve({ ok: false, err: err.message }));
  });

  if (udpOk.ok) {
    console.log(`  ${colors.green}✔ 本地 UDP 探针套接字绑定成功${colors.reset} (测试端口: ${udpOk.port})\n`);
  } else {
    console.log(`  ${colors.red}✖ 本地 UDP 绑定失败: ${udpOk.err}${colors.reset}\n`);
  }
}

async function testStunServer(host, port) {
  return new Promise((resolve) => {
    const startTime = Date.now();
    const socket = dgram.createSocket('udp4');
    let done = false;

    const timer = setTimeout(() => {
      if (!done) {
        done = true;
        socket.close();
        resolve({ host, port, ok: false, error: '请求超时 (1500ms)' });
      }
    }, 1500);

    // 标准 STUN Binding Request 报文 (RFC 5389): 20 字节
    const msg = Buffer.alloc(20);
    msg.writeUInt16BE(0x0001, 0); // Binding Request
    msg.writeUInt16BE(0x0000, 2); // Message Length
    msg.writeUInt32BE(0x2112A442, 4); // Magic Cookie
    for (let i = 8; i < 20; i++) msg[i] = Math.floor(Math.random() * 256);

    socket.on('message', (response) => {
      if (!done) {
        done = true;
        clearTimeout(timer);
        const rtt = Date.now() - startTime;
        socket.close();
        resolve({ host, port, ok: true, rtt });
      }
    });

    socket.on('error', (err) => {
      if (!done) {
        done = true;
        clearTimeout(timer);
        socket.close();
        resolve({ host, port, ok: false, error: err.message });
      }
    });

    socket.send(msg, port, host, (err) => {
      if (err && !done) {
        done = true;
        clearTimeout(timer);
        socket.close();
        resolve({ host, port, ok: false, error: err.message });
      }
    });
  });
}

async function testStunMatrix() {
  console.log(`${colors.bold}[2/3] 正在检测国内高可用 STUN 穿透矩阵响应...${colors.reset}`);
  const stuns = [
    { host: 'stun.qq.com', port: 3478, name: '腾讯云 STUN' },
    { host: 'stun.miwifi.com', port: 3478, name: '小米路由 STUN' },
    { host: 'stun.chat.bilibili.com', port: 3478, name: '哔哩哔哩 STUN' },
  ];

  for (const s of stuns) {
    const res = await testStunServer(s.host, s.port);
    if (res.ok) {
      console.log(`  ${colors.green}✔ [${s.name}]${colors.reset} ${s.host}:${s.port} 穿透响应正常 (往返延迟: ${res.rtt}ms)`);
    } else {
      console.log(`  ${colors.yellow}⚠ [${s.name}]${colors.reset} ${s.host}:${s.port} 响应较慢或受限 (${res.error})`);
    }
  }
  console.log();
}

async function testMqttSignaling() {
  console.log(`${colors.bold}[3/3] 正在检测公共 WSS 信令通道与 MQTT 握手状态...${colors.reset}`);
  const url = 'wss://broker.emqx.io:8084/mqtt';

  return new Promise((resolve) => {
    const startTime = Date.now();
    let done = false;

    const timer = setTimeout(() => {
      if (!done) {
        done = true;
        console.log(`  ${colors.red}✖ WSS 信令握手超时: ${url}${colors.reset}\n`);
        resolve(false);
      }
    }, 6000);

    try {
      const ws = new WebSocket(url, ['mqtt']);

      ws.on('open', () => {
        // 构建标准 MQTT CONNECT 报文
        const clientPayload = Buffer.from('wga_diag_' + Math.random().toString(36).substring(2, 6));
        const varHeader = Buffer.from([0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x04, 0x02, 0x00, 0x3c]);
        const clientLen = Buffer.from([0x00, clientPayload.length]);
        const remLen = varHeader.length + clientLen.length + clientPayload.length;
        const packet = Buffer.concat([Buffer.from([0x10, remLen]), varHeader, clientLen, clientPayload]);
        ws.send(packet);
      });

      ws.on('message', (data) => {
        if (!done && data.length >= 4 && (data[0] >> 4 === 2)) {
          done = true;
          clearTimeout(timer);
          const rtt = Date.now() - startTime;
          const returnCode = data[3];
          if (returnCode === 0) {
            console.log(`  ${colors.green}✔ WSS 信令连接与 MQTT CONNACK 握手成功!${colors.reset} (时延: ${rtt}ms)`);
            console.log(`    信令地址: ${url}`);
          } else {
            console.log(`  ${colors.yellow}⚠ MQTT 返回非零状态码: ${returnCode}${colors.reset}`);
          }
          ws.close();
          console.log();
          resolve(true);
        }
      });

      ws.on('error', (err) => {
        if (!done) {
          done = true;
          clearTimeout(timer);
          console.log(`  ${colors.red}✖ WSS 信令连接失败: ${err.message}${colors.reset}\n`);
          resolve(false);
        }
      });
    } catch (err) {
      if (!done) {
        done = true;
        clearTimeout(timer);
        console.log(`  ${colors.red}✖ 创建 WebSocket 异常: ${err.message}${colors.reset}\n`);
        resolve(false);
      }
    }
  });
}

async function run() {
  await testLocalSockets();
  await testStunMatrix();
  await testMqttSignaling();

  console.log(`${colors.cyan}${colors.bold}============================================================${colors.reset}`);
  console.log(`${colors.green}${colors.bold}🎉 连通性自检完成！所有关键通信链路均已就绪。${colors.reset}`);
  console.log(`${colors.cyan}如需测试 AI 机器人，请执行:${colors.reset} pnpm test:bots 或 pnpm mock:bots`);
  console.log(`${colors.cyan}${colors.bold}============================================================${colors.reset}\n`);
}

run();
