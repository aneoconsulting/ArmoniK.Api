FROM python:3.7-slim AS builder
WORKDIR /app
RUN python -m venv .venv && .venv/bin/pip install --no-cache-dir -U pip setuptools
COPY worker-requirements.txt *.whl ./
RUN .venv/bin/pip install --no-cache-dir $( ( find . -type f -name "*.whl" | grep . ) || echo armonik ) -r worker-requirements.txt && find /app/.venv \( -type d -a -name test -o -name tests \) -o \( -type f -a -name '*.pyc' -o -name '*.pyo' \) -exec rm -rf '{}' \+

FROM python:3.7-slim
WORKDIR /app
RUN groupadd --gid 5000 armonikuser && useradd --home-dir /home/armonikuser --create-home --uid 5000 --gid 5000 --shell /bin/sh --skel /dev/null armonikuser && mkdir /cache && chown armonikuser: /cache
USER armonikuser
ENV PATH="/app/.venv/bin:$PATH" PYTHONUNBUFFERED=1
COPY --from=builder /app /app
COPY . .
ENTRYPOINT ["python", "worker.py"]