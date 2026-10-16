import { Command } from "commander";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { SolPatronWorker } from "./worker";

const program = new Command();

program
  .name("solpatron-cli")
  .description("Enterprise CLI for managing SolPatron creator subscriptions on Solana")
  .version("1.0.0");

program
  .command("verify-access")
  .description("Cryptographically verify if a wallet has active access to a creator's tier")
  .requiredOption("-w, --wallet <pubkey>", "Subscriber wallet public key")
  .requiredOption("-t, --tier <pubkey>", "Tier public key")
  .action(async (opts) => {
    const rpc = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
    const conn = new Connection(rpc, "confirmed");
    const programId = new PublicKey("SPatrn1111111111111111111111111111111111111");

    const [subPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("subscription"), new PublicKey(opts.tier).toBuffer(), new PublicKey(opts.wallet).toBuffer()],
      programId
    );

    const info = await conn.getAccountInfo(subPda);
    if (!info) {
      console.log(JSON.stringify({ active: false, reason: "NO_SUBSCRIPTION_FOUND" }));
      return;
    }

    const data = info.data;
    const validUntil = Number(data.readBigInt64LE(8 + 32 + 32 + 32 + 8 + 8 + 8));
    const now = Math.floor(Date.now() / 1000);
    const active = validUntil > now;

    console.log(
      JSON.stringify(
        {
          subscriptionPda: subPda.toBase58(),
          subscriber: opts.wallet,
          validUntil: new Date(validUntil * 1000).toISOString(),
          active,
        },
        null,
        2
      )
    );
  });

program
  .command("trigger-webhook-test")
  .description("Send mock test dispatch to Resend & Discord")
  .requiredOption("-e, --email <email>", "Destination email")
  .action(async (opts) => {
    const worker = new SolPatronWorker();
    console.log("[CLI] Triggering enterprise notification test...");
    const resendSuccess = await worker.notifySubscriberViaResend(
      opts.email,
      "VIP Backstage Access",
      new Date(Date.now() + 30 * 86400 * 1000).toUTCString()
    );
    const discordSuccess = await worker.postDiscordSubscriptionEvent(
      "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
      "VIP Backstage Access",
      "15.00"
    );
    console.log(`[CLI] Results: Resend=${resendSuccess}, Discord=${discordSuccess}`);
  });

program.parse(process.argv);