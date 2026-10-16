import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Solpatron } from "../target/types/solpatron";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert, expect } from "chai";

describe("SolPatron: Comprehensive Subscription Suite", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Solpatron as Program<Solpatron>;

  const admin = anchor.web3.Keypair.generate();
  const feeRecipient = anchor.web3.Keypair.generate();
  const creator = anchor.web3.Keypair.generate();
  const subscriber = anchor.web3.Keypair.generate();

  let paymentMint: anchor.web3.PublicKey;
  let creatorTokenAccount: anchor.web3.PublicKey;
  let subscriberTokenAccount: anchor.web3.PublicKey;
  let feeRecipientTokenAccount: anchor.web3.PublicKey;

  let protocolConfigPda: anchor.web3.PublicKey;
  let creatorProfilePda: anchor.web3.PublicKey;
  let tierPda: anchor.web3.PublicKey;
  let subscriptionPda: anchor.web3.PublicKey;
  let escrowVaultPda: anchor.web3.PublicKey;

  const TIER_ID = 1;
  const COST_PER_PERIOD = new anchor.BN(10_000_000); // 10 USDC (6 Decimals)
  const DURATION_SECONDS = new anchor.BN(86_400); // 1 Day

  before(async () => {
    // Airdrop SOL
    for (const kp of [admin, creator, subscriber]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }

    // Create Payment Mint & Accounts
    paymentMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    creatorTokenAccount = await createAccount(
      provider.connection,
      creator,
      paymentMint,
      creator.publicKey
    );

    subscriberTokenAccount = await createAccount(
      provider.connection,
      subscriber,
      paymentMint,
      subscriber.publicKey
    );

    feeRecipientTokenAccount = await createAccount(
      provider.connection,
      admin,
      paymentMint,
      feeRecipient.publicKey
    );

    // Mint funds to subscriber (100 USDC)
    await mintTo(
      provider.connection,
      admin,
      paymentMint,
      subscriberTokenAccount,
      admin,
      100_000_000
    );

    // Derive PDAs
    [protocolConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("protocol_config")],
      program.programId
    );

    [creatorProfilePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("creator"), creator.publicKey.toBuffer()],
      program.programId
    );

    [tierPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("tier"),
        creatorProfilePda.toBuffer(),
        Buffer.from([TIER_ID]),
      ],
      program.programId
    );

    [subscriptionPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("subscription"),
        tierPda.toBuffer(),
        subscriber.publicKey.toBuffer(),
      ],
      program.programId
    );

    [escrowVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), subscriptionPda.toBuffer()],
      program.programId
    );
  });

  it("1. Initializes global protocol configuration", async () => {
    await program.methods
      .initializeProtocol(feeRecipient.publicKey)
      .accounts({
        config: protocolConfigPda,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const config = await program.account.protocolConfig.fetch(protocolConfigPda);
    assert.equal(config.admin.toBase58(), admin.publicKey.toBase58());
    assert.equal(config.feeRecipient.toBase58(), feeRecipient.publicKey.toBase58());
    assert.equal(config.feeBps.toNumber(), 50);
  });

  it("2. Registers a new creator profile", async () => {
    await program.methods
      .initializeCreator("https://arweave.net/solpatron-creator-metadata")
      .accounts({
        creatorProfile: creatorProfilePda,
        config: protocolConfigPda,
        paymentMint: paymentMint,
        creator: creator.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    const profile = await program.account.creatorProfile.fetch(creatorProfilePda);
    assert.equal(profile.creatorAuthority.toBase58(), creator.publicKey.toBase58());
    assert.equal(profile.tierCount, 0);
  });

  it("3. Creates a new tier with valid pricing", async () => {
    await program.methods
      .createTier(TIER_ID, COST_PER_PERIOD, DURATION_SECONDS, "VIP Patron")
      .accounts({
        tier: tierPda,
        creatorProfile: creatorProfilePda,
        creatorAuthority: creator.publicKey,
        creator: creator.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    const tier = await program.account.subscriptionTier.fetch(tierPda);
    assert.equal(tier.name, "VIP Patron");
    assert.isTrue(tier.costPerPeriod.eq(COST_PER_PERIOD));
  });

  it("4. Rejects tier creation with invalid (zero) cost", async () => {
    const invalidTierId = 99;
    const [invalidTierPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("tier"),
        creatorProfilePda.toBuffer(),
        Buffer.from([invalidTierId]),
      ],
      program.programId
    );

    try {
      await program.methods
        .createTier(invalidTierId, new anchor.BN(0), DURATION_SECONDS, "Zero Cost")
        .accounts({
          tier: invalidTierPda,
          creatorProfile: creatorProfilePda,
          creatorAuthority: creator.publicKey,
          creator: creator.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([creator])
        .rpc();
      assert.fail("Should have rejected zero cost tier");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidTierCost");
    }
  });

  it("5. Subscriber funds a 2-period subscription into PDA escrow", async () => {
    const periods = new anchor.BN(2);
    await program.methods
      .subscribe(periods)
      .accounts({
        subscription: subscriptionPda,
        escrowVault: escrowVaultPda,
        creatorProfile: creatorProfilePda,
        tier: tierPda,
        paymentMint: paymentMint,
        subscriberTokenAccount: subscriberTokenAccount,
        subscriber: subscriber.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([subscriber])
      .rpc();

    const sub = await program.account.subscription.fetch(subscriptionPda);
    assert.isTrue(sub.isActive);
    assert.isTrue(sub.escrowBalance.eq(new anchor.BN(20_000_000)));

    const vaultAccount = await getAccount(provider.connection, escrowVaultPda);
    assert.equal(vaultAccount.amount.toString(), "20000000");
  });

  it("6. Autonomous crank settles vested streaming balance", async () => {
    // Settle unlocked portion
    await program.methods
      .settleSubscription()
      .accounts({
        subscription: subscriptionPda,
        escrowVault: escrowVaultPda,
        tier: tierPda,
        creatorProfile: creatorProfilePda,
        creatorTokenAccount: creatorTokenAccount,
        protocolFeeTokenAccount: feeRecipientTokenAccount,
        config: protocolConfigPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const sub = await program.account.subscription.fetch(subscriptionPda);
    assert.isTrue(sub.isActive);
  });

  it("7. Subscriber cancels remaining unvested balance and receives refund", async () => {
    const subBefore = await getAccount(provider.connection, subscriberTokenAccount);

    await program.methods
      .cancelSubscription()
      .accounts({
        subscription: subscriptionPda,
        escrowVault: escrowVaultPda,
        tier: tierPda,
        creatorProfile: creatorProfilePda,
        creatorTokenAccount: creatorTokenAccount,
        subscriberTokenAccount: subscriberTokenAccount,
        subscriber: subscriber.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([subscriber])
      .rpc();

    const sub = await program.account.subscription.fetch(subscriptionPda);
    assert.isFalse(sub.isActive);
    assert.equal(sub.escrowBalance.toNumber(), 0);

    const subAfter = await getAccount(provider.connection, subscriberTokenAccount);
    assert.isTrue(BigInt(subAfter.amount) > BigInt(subBefore.amount));
  });
});