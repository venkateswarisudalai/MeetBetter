# Vantage API Proxy

A Cloudflare Worker that proxies Deepgram and Groq API calls so users can try Vantage without their own API keys. Includes per-IP daily rate limiting.

## Setup

### 1. Install Wrangler

```bash
npm install
```

### 2. Create KV namespace for rate limiting

```bash
npx wrangler kv namespace create RATE_LIMIT
```

Copy the output `id` into `wrangler.toml` under `[[kv_namespaces]]`.

### 3. Set your API keys as secrets

```bash
npx wrangler secret put GROQ_API_KEY
npx wrangler secret put DEEPGRAM_API_KEY
npx wrangler secret put DEEPGRAM_PROJECT_ID
```

- **GROQ_API_KEY**: Get from https://console.groq.com
- **DEEPGRAM_API_KEY**: Get from https://console.deepgram.com
- **DEEPGRAM_PROJECT_ID**: Found in Deepgram Console → Settings → Project

### 4. Deploy

```bash
npm run deploy
```

Your proxy will be live at `https://vantage-api-proxy.<your-subdomain>.workers.dev`.

### 5. Configure rate limits

Edit `wrangler.toml` to adjust:
- `DAILY_LIMIT` — max Groq calls per IP per day (default: 50)
- `DAILY_DEEPGRAM_TOKEN_LIMIT` — max Deepgram temp keys per IP per day (default: 10)

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/deepgram/token` | Returns a temporary Deepgram API key (5 min TTL) |
| POST | `/api/groq/chat` | Proxies to Groq chat completions |
| POST | `/api/groq/whisper` | Proxies to Groq Whisper transcription |
| GET | `/health` | Health check |

## Cost Control

- Deepgram: Set a spending limit in the Deepgram Console under Billing
- Groq: Free tier has generous limits; set spending alerts in Console
- Proxy: Cloudflare Workers free tier allows 100K requests/day
- Rate limiting prevents any single IP from using more than the configured daily limits
