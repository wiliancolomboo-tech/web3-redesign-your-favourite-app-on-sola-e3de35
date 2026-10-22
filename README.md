# Jupiter Next: Next-Gen Solana Swap & Trading Terminal UX Redesign

> **Submission for Superteam Open Design Bounty 2026: "Redesign Your Favourite App on Solana"**  
> **Platform / Category:** Product Design, UX Architecture, Front-End Live Prototype  
> **Ecosystem:** Solana Mainnet-ready  
> **Payout Address (Solana SPL / USDC):** `7hn8kUNbVfbjHF7FrYuFs8wdvhHXajte8aC2ud1mWDCH`  
> **Live Interactive Prototype:** [https://wiliancolomboo-tech.github.io/superteam-jupiter-ux-redesign/](https://wiliancolomboo-tech.github.io/superteam-jupiter-ux-redesign/)

---

## 1. Executive Summary & Design Thesis

Solana has achieved unprecedented throughput (>2,800 TPS) and sub-second settlement, establishing itself as the premier execution layer for decentralized finance. At the epicenter of this activity sits **Jupiter Exchange**, routing over $1B+ in daily volume across hundreds of thousands of users.

However, as decentralized finance transitions from crypto-native degens to mainstream global adoption, decentralized exchange interfaces suffer from systemic UX debt inherited from Ethereum's high-gas era:
1. **Cryptic Failure Modes:** Users are confronted with raw hexadecimal simulation errors (e.g. `Custom Program Error 0x1771`) when slippage occurs, creating panic and redundant gas burning.
2. **Invisible Value Extraction (MEV):** Retail traders placing $200–$2,000 swaps without custom priority configurations are routinely sandwiched by searchers, losing up to 2-3% without understanding why their realized output deviated from the quote.
3. **Trapped Capital in Inactive Accounts:** Solana's rent-exemption model requires ~0.00203928 SOL per associated token account (ATA). The average trader unknowingly locks up $10–$50 across dozens of traded pairs with zero native discovery or recovery mechanism in the primary trade flow.
4. **Cognitive Overload for Beginners:** Presenting raw Compute Unit limits, Basis Points (BPS), and complex routing graphs upfront scares retail users away from on-chain liquidity.

### The Solution: Jupiter Next
**Jupiter Next** reimagines the decentralized exchange experience through **Predictive Protection, Dynamic Route Transparency, and Progressive Disclosure**. It delivers a single responsive application that effortlessly serves both first-time retail swappers and high-frequency quant traders through an intelligent Dual-Mode architecture.

---

## 2. Before vs. After: The 4 Core UX Transformations

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                               UX PARADIGM SHIFT                                        │
├─────────────────────────────────────┬──────────────────────────────────────────────────┤
│ LEGACY JUPITER EXPERIENCE           │ JUPITER NEXT REDESIGNED EXPERIENCE               │
├─────────────────────────────────────┼──────────────────────────────────────────────────┤
│ 1. Cryptic Error Codes              │ 1. Predictive Pre-Flight Human Guidance          │
│    "Program Error: 0x1771"          │    Clear volatility warnings & 1-click fallback  │
│                                     │                                                  │
│ 2. Invisible MEV Extraction         │ 2. Real-Time MEV Risk Barometer                  │
│    Sandwiched by searchers          │    Dynamic Jito bundle tips & visual hazard gauge│
│                                     │                                                  │
│ 3. Trapped Rent Capital             │ 3. 1-Click Rent Reclaim Harvester                │
│    SOL buried in dormant ATAs       │    Persistent header banner displaying idle SOL  │
│                                     │                                                  │
│ 4. Complex BPS & CU Jargon          │ 4. Dual-Mode UI Architecture                     │
│    Intimidating to retail           │    Instant Swap (Novice) vs Pro Quant Terminal   │
└─────────────────────────────────────┴──────────────────────────────────────────────────┘
```

### Feature Breakdown

### 🛡️ Feature 1: Predictive MEV Barometer & Dynamic Jito Shield
* **Problem:** Retail swappers do not understand MEV or priority fees. They either underpay and get sandwiched, or massively overpay fixed tips.
* **Redesign Solution:** A live hazard barometer continuously evaluates the input trade size relative to pool liquidity depth. When potential sandwich extraction exceeds the network fee, the interface automatically proposes a dynamic Jito bundle tip computed over the median cluster tip of the last 150 slots, guaranteeing inclusion in the very next block without overpaying.

### 🧭 Feature 2: Transparent Split Route Visualizer
* **Problem:** Existing route visualizers feel like a black box. Users see percentages without understanding the economic benefit.
* **Redesign Solution:** A visual pipeline displays the split across liquidity venues (e.g. 60% Raydium CLMM + 25% Orca Whirlpool + 15% Phoenix CLOB) alongside the real dollar savings compared to centralized exchanges, confirming that Jupiter genuinely delivered the best on-chain price.

### 💰 Feature 3: Native Rent Account Harvester
* **Problem:** Users accumulate dead associated token accounts across meme coins and experimental tokens, stranding SOL.
* **Redesign Solution:** A persistent, non-intrusive banner in the top navigation detects empty token accounts and provides an instant one-tap claim action to burn the empty ATA and reclaim the 0.00203928 SOL rent directly to the user's wallet.

### ⚡ Feature 4: Dual-Mode UI (Instant vs. Pro Quant)
* **Problem:** One-size-fits-all layouts satisfy neither beginners (who are overwhelmed) nor professional traders (who lack deep orderbook depth and microstructure).
* **Redesign Solution:**
  * **Instant Mode:** Ultra-clean card interface with human-readable dollars, automated slippage floor (0.10%), zero technical jargon, and large touch targets.
  * **Pro Quant Mode:** Reveals live Order Book Imbalance (OBI L1-L5), tick liquidity concentration, and real-time bid/ask pressure ratios across Phoenix and OpenBook.

---

## 3. Responsive Architecture: Desktop & Mobile Optimization

The interface was designed mobile-first and tested across both viewports:
* **Desktop (1280px+):** Expansive 12-column layout pairing deep route analytics and market microstructure on the left with the high-conversion swap terminal on the right.
* **Mobile (360px - 420px):** Single-column stacked workflow featuring thumb-friendly interaction zones, fixed bottom action buttons, and slide-over sheets for token selection and settings.

---

## 4. Verification & Interactive Live Prototype

Unlike static Figma mockups, this deliverable includes a fully interactive, production-grade single-page application built with **Tailwind CSS, HTML5, and Lucide Icons**:
* **Live Demo URL:** [https://wiliancolomboo-tech.github.io/superteam-jupiter-ux-redesign/](https://wiliancolomboo-tech.github.io/superteam-jupiter-ux-redesign/)
* **Local Run:** Simply open `index.html` in any modern web browser or serve via `python3 -m http.server 8080`.

### Key Interactive Flows to Test in the Prototype:
1. **Toggle Before/After:** Switch between "Redesigned Experience" and "Legacy UX Pain Points" to inspect the exact audit matrix.
2. **Toggle Instant vs. Pro Quant:** Notice how advanced market depth and OBI telemetry smoothly disclose without breaking layout.
3. **Simulate Trade Size Changes:** Type `5.0`, `20.0`, and `60.0` SOL in the input field to watch the MEV Risk Barometer dynamically shift between **Safe**, **Moderate**, and **High Hazard**.
4. **Device Switcher:** Click the mobile phone icon in the top header to simulate responsive viewport constraints in real time.

---

## 5. Payout & Submission Metadata

* **Bounty Listing:** Superteam Earn Open Design Bounty (`redesign-your-favourite-app-on-solana`)
* **Prize Pool:** $1,500 USDC ($700 1st, $500 2nd, $300 3rd)
* **Official Settlement Address (Solana Mainnet):**  
  `7hn8kUNbVfbjHF7FrYuFs8wdvhHXajte8aC2ud1mWDCH`
* **Author / Submitter:** Wilian Colombo (`wiliancolomboo-tech`)
