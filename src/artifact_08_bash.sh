# 1. Clone repository & install dependencies
git clone https://github.com/solpatron/solpatron-protocol.git
cd solpatron-protocol
npm install

# 2. Build Anchor Smart Contracts
anchor build

# 3. Run full automated test suite on local validator
anchor test