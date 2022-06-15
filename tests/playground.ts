import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { getAccount } from "@solana/spl-token";
import { BN } from "bn.js";
import { assert } from "chai";

import { Escrow } from "../target/types/escrow";
import {
  createAssociatedTokenAccount,
  createFundedWallet,
  createMint,
} from "./utils";

describe("playground", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Escrow as Program<Escrow>;
  const mintDecimals = 6;
  let offeredMint: anchor.web3.PublicKey;
  let requestedMint: anchor.web3.PublicKey;
  const firstTrade = anchor.web3.Keypair.generate();
  const secondTrade = anchor.web3.Keypair.generate();

  let alice: anchor.web3.Keypair;
  let aliceOfferedVault: anchor.web3.PublicKey;
  let aliceRequestedVault: anchor.web3.PublicKey;
  const aliceOfferedBalance = 1000;
  const aliceRequestedBalance = 1200;
  let bob: anchor.web3.Keypair;
  let bobOfferedVault: anchor.web3.PublicKey;
  let bobRequestedVault: anchor.web3.PublicKey;
  const bobOfferedBalance = 2000;
  const bobRequestedBalance = 3200;

  before(async () => {
    offeredMint = await createMint(provider, mintDecimals);
    requestedMint = await createMint(provider, mintDecimals);
    alice = await createFundedWallet(provider);
    aliceOfferedVault = await createAssociatedTokenAccount(
      provider,
      offeredMint,
      BigInt(`0x${new anchor.BN(aliceOfferedBalance).toString("hex")}`),
      alice
    );
    aliceRequestedVault = await createAssociatedTokenAccount(
      provider,
      requestedMint,
      BigInt(`0x${new anchor.BN(aliceRequestedBalance).toString("hex")}`),
      alice
    );
    bob = await createFundedWallet(provider);
    bobOfferedVault = await createAssociatedTokenAccount(
      provider,
      offeredMint,
      BigInt(`0x${new anchor.BN(bobOfferedBalance).toString("hex")}`),
      bob
    );
    bobRequestedVault = await createAssociatedTokenAccount(
      provider,
      requestedMint,
      BigInt(`0x${new anchor.BN(bobRequestedBalance).toString("hex")}`),
      bob
    );
  });

  it("should start a trade", async () => {
    // arrange
    const amountOffered = 1;
    const amountRequested = 2;
    const [firstTradePublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade", "utf-8"), firstTrade.publicKey.toBuffer()],
        program.programId
      );
    const [firstTradeVaultPublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade_vault", "utf-8"), firstTradePublicKey.toBuffer()],
        program.programId
      );
    const authorVaultAccountBefore = await getAccount(
      provider.connection,
      aliceOfferedVault
    );
    // act
    await program.methods
      .startTrade(new BN(amountOffered), new BN(amountRequested))
      .accounts({
        authority: alice.publicKey,
        base: firstTrade.publicKey,
        authorVault: aliceOfferedVault,
        mintOffered: offeredMint,
        mintRequested: requestedMint,
      })
      .signers([alice])
      .rpc();
    // assert
    const tradeAccount = await program.account.trade.fetch(firstTradePublicKey);
    const authorVaultAccountAfter = await getAccount(
      provider.connection,
      aliceOfferedVault
    );
    const tradeVaultAccount = await getAccount(
      provider.connection,
      firstTradeVaultPublicKey
    );
    assert.isTrue(tradeAccount.amountOffered.eq(new BN(amountOffered)));
    assert.isTrue(tradeAccount.amountRequested.eq(new BN(amountRequested)));
    assert.isFalse(tradeAccount.executed);
    assert.equal(
      tradeVaultAccount.amount,
      BigInt(`0x${new BN(amountOffered).toString("hex")}`)
    );
    assert.equal(
      authorVaultAccountBefore.amount,
      authorVaultAccountAfter.amount + tradeVaultAccount.amount
    );
  });

  it("should execute a trade", async () => {
    // arrange
    const [firstTradePublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade", "utf-8"), firstTrade.publicKey.toBuffer()],
        program.programId
      );
    const [firstTradeVaultPublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade_vault", "utf-8"), firstTradePublicKey.toBuffer()],
        program.programId
      );
    const authorRequestedVaultAccountBefore = await getAccount(
      provider.connection,
      aliceRequestedVault
    );
    const executerOfferedVaultAccountBefore = await getAccount(
      provider.connection,
      bobOfferedVault
    );
    const executerRequestedVaultAccountBefore = await getAccount(
      provider.connection,
      bobRequestedVault
    );
    // act
    await program.methods
      .executeTrade()
      .accounts({
        authority: bob.publicKey,
        base: firstTrade.publicKey,
        authorRequestedVault: aliceRequestedVault,
        executerOfferedVault: bobOfferedVault,
        executerRequestedVault: bobRequestedVault,
      })
      .signers([bob])
      .rpc();
    // assert
    const tradeAccount = await program.account.trade.fetch(firstTradePublicKey);
    const authorRequestedVaultAccountAfter = await getAccount(
      provider.connection,
      aliceRequestedVault
    );
    const executerOfferedVaultAccountAfter = await getAccount(
      provider.connection,
      bobOfferedVault
    );
    const executerRequestedVaultAccountAfter = await getAccount(
      provider.connection,
      bobRequestedVault
    );
    const tradeVaultAccount = await getAccount(
      provider.connection,
      firstTradeVaultPublicKey
    );
    assert.isTrue(tradeAccount.executed);
    assert.equal(tradeVaultAccount.amount, BigInt(`0x0`));
    assert.equal(
      executerOfferedVaultAccountAfter.amount,
      executerOfferedVaultAccountBefore.amount +
        BigInt(`0x${new BN(tradeAccount.amountOffered).toString("hex")}`)
    );
    assert.equal(
      executerRequestedVaultAccountAfter.amount,
      executerRequestedVaultAccountBefore.amount -
        BigInt(`0x${new BN(tradeAccount.amountRequested).toString("hex")}`)
    );
    assert.equal(
      authorRequestedVaultAccountAfter.amount,
      authorRequestedVaultAccountBefore.amount +
        BigInt(`0x${new BN(tradeAccount.amountRequested).toString("hex")}`)
    );
  });

  it("should delete a trade", async () => {
    // arrange
    const [firstTradePublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade", "utf-8"), firstTrade.publicKey.toBuffer()],
        program.programId
      );
    const [firstTradeVaultPublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade_vault", "utf-8"), firstTradePublicKey.toBuffer()],
        program.programId
      );
    // act
    await program.methods
      .deleteTrade()
      .accounts({
        authority: alice.publicKey,
        base: firstTrade.publicKey,
      })
      .signers([alice])
      .rpc();
    // assert
    const tradeAccount = await program.account.trade.fetchNullable(
      firstTradePublicKey
    );
    const tradeVaultAccount = await provider.connection.getAccountInfo(
      firstTradeVaultPublicKey
    );
    assert.isNull(tradeAccount);
    assert.isNull(tradeVaultAccount);
  });

  it("should cancel a trade", async () => {
    // arrange
    const amountOffered = 1;
    const amountRequested = 2;
    const [secondTradePublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade", "utf-8"), secondTrade.publicKey.toBuffer()],
        program.programId
      );
    const [secondTradeVaultPublicKey] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("trade_vault", "utf-8"), secondTradePublicKey.toBuffer()],
        program.programId
      );
    // act
    await program.methods
      .startTrade(new BN(amountOffered), new BN(amountRequested))
      .accounts({
        authority: alice.publicKey,
        base: secondTrade.publicKey,
        authorVault: aliceOfferedVault,
        mintOffered: offeredMint,
        mintRequested: requestedMint,
      })
      .signers([alice])
      .rpc();
    await program.methods
      .cancelTrade()
      .accounts({
        authority: alice.publicKey,
        base: secondTrade.publicKey,
        authorVault: aliceOfferedVault,
      })
      .signers([alice])
      .rpc();
    // assert
    const tradeAccount = await program.account.trade.fetchNullable(
      secondTradePublicKey
    );
    const tradeVaultAccount = await provider.connection.getAccountInfo(
      secondTradeVaultPublicKey
    );
    assert.isNull(tradeAccount);
    assert.isNull(tradeVaultAccount);
  });
});
