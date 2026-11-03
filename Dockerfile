# ==========================================
# 卧底猜词助手 (Word-Guess-Assistant) Web 容器镜像
# Multi-stage Dockerfile for Web Hosting
# ==========================================

# 阶段 1: Node 编译静态资源
FROM node:22-alpine AS builder

WORKDIR /app

# 安装 pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# 优先缓存依赖层
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

# 拷贝源代码并执行 Web 构建
COPY . .
RUN pnpm build

# 阶段 2: Nginx 极简运行镜像
FROM nginx:alpine-slim

# 拷贝定制 Nginx 配置
COPY nginx.conf /etc/nginx/conf.d/default.conf

# 拷贝前端构建产物
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

