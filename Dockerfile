FROM python:3.12-slim

# certifi：容器内自带 CA 信任包，脚本会自动优先使用
# YUJIA_WEB=1：容器内同时运行 Web 面板（compose 映射到宿主机 127.0.0.1）
ENV TZ=Asia/Shanghai \
    PYTHONUNBUFFERED=1 \
    YUJIA_CONFIG_DIR=/data \
    YUJIA_WEB=1

WORKDIR /app
COPY core/yujia_signin.py core/signin_core.py core/web_panel.py core/run_daily.py ./

RUN pip install --no-cache-dir certifi "qrcode>=8,<9" && mkdir -p /data
VOLUME /data

# 长驻容器：内部调度器每天 SIGNIN_TIME 执行签到 + Web 面板线程，日志走 docker logs
CMD ["python", "run_daily.py"]
