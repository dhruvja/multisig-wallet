import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Project } from "../target/types/project";
import { General } from "../target/types/general";
import {Transfer} from "../target/types/transfer";
const assert = require("assert");
import * as spl from "@solana/spl-token";
import bs58 from "bs58";
import { v4 as uuidv4 } from "uuid";

describe("project", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();

  anchor.setProvider(provider);

  const projectProgram = anchor.workspace.Project as Program<Project>;
  const generalProgram = anchor.workspace.General as Program<General>;
  const transferProgram = anchor.workspace.Transfer as Program<Transfer>;

  let alice: anchor.web3.Keypair;
  let bob: anchor.web3.Keypair;
  let cas: anchor.web3.Keypair;
  let dan: anchor.web3.Keypair;
  let admin: anchor.web3.Keypair;

  let USDCMint: anchor.web3.PublicKey; // token which would be staked
  let casTokenAccount: any; // cas token account

  let initialMintAmount = 100000000;

  alice = anchor.web3.Keypair.generate();
  bob = anchor.web3.Keypair.generate();
  cas = anchor.web3.Keypair.generate();
  dan = anchor.web3.Keypair.generate();
  admin = anchor.web3.Keypair.generate(); // Admin

  const threshold = 2;
  const timeLimit = 100 * 60 * 60 * 24; // 1 day

  const transferAmount1 = 1000;

  it("Funds all users", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(alice.publicKey, 10000000000),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(bob.publicKey, 10000000000),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(cas.publicKey, 10000000000),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(dan.publicKey, 10000000000),
      "confirmed"
    );

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(admin.publicKey, 10000000000),
      "confirmed"
    );

    const aliceUserBalance = await provider.connection.getBalance(
      alice.publicKey
    );
    const bobUserBalance = await provider.connection.getBalance(bob.publicKey);
    const casUserBalance = await provider.connection.getBalance(cas.publicKey);
    const danUserBalance = await provider.connection.getBalance(dan.publicKey);
    const adminUserBalance = await provider.connection.getBalance(
      admin.publicKey
    );

    assert.strictEqual(10000000000, aliceUserBalance);
    assert.strictEqual(10000000000, bobUserBalance);
    assert.strictEqual(10000000000, casUserBalance);
    assert.strictEqual(10000000000, danUserBalance);
    assert.strictEqual(10000000000, adminUserBalance);
  });

  it("create USDC mint and mint some tokens to stakeholders", async () => {
    USDCMint = await spl.createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    casTokenAccount = await spl.createAccount(
      provider.connection,
      cas,
      USDCMint,
      cas.publicKey
    );

    await spl.mintTo(
      provider.connection,
      cas,
      USDCMint,
      casTokenAccount,
      admin.publicKey,
      initialMintAmount,
      [admin]
    );

    let _casTokenAccount = await spl.getAccount(
      provider.connection,
      casTokenAccount
    );

    assert.equal(initialMintAmount, _casTokenAccount.amount);
  });

  it("Initialize general program", async () => {
    const [generalPDA, generalBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("general")],
        generalProgram.programId
      );

    const tx = await generalProgram.methods
      .initialize()
      .accounts({
        baseAccount: generalPDA,
        authority: admin.publicKey,
        tokenMint: USDCMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const state = await generalProgram.account.generalParameter.fetch(generalPDA);
    assert.equal(state.tokenMint.toBase58(), USDCMint.toBase58())

    try {
      await generalProgram.methods.changeMint(generalBump).accounts({
        baseAccount: generalPDA,
        tokenMint: USDCMint,
        authority: alice.publicKey
      }).signers([alice]).rpc();

      throw "Error occured, invalid authority while initializing general program"
    } catch (error) {
      assert.equal(error.error.errorCode.number, 2001);
    }
  });

  const projectId = uuidv4();
  const transferId = uuidv4();

  it("initializes project program", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("project"), Buffer.from(projectId.substring(0,18)), Buffer.from(projectId.substring(18,36))],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .initialize(projectId)
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([alice])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    // assert.equal(state.signatories, []);

    assert.equal(state.add.status, false);
    assert.equal(state.add.votes, 0);
    assert.equal(state.add.timestamp, 0);

    assert.equal(state.delete.status, false);
    assert.equal(state.delete.votes, 0);
    assert.equal(state.delete.timestamp, 0);

    assert.equal(state.changeThreshold.status, false);
    assert.equal(state.changeThreshold.votes, 0);
    assert.equal(state.changeThreshold.timestamp, 0);
    assert.equal(state.changeThreshold.newThreshold, 0);

    assert.equal(state.threshold, 0);
  });

  it("Initialze the signatories for the project", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("project"), Buffer.from(projectId.substring(0,18)), Buffer.from(projectId.substring(18,36))],
        projectProgram.programId
      );

    const all = [
      admin.publicKey,
      alice.publicKey,
      bob.publicKey,
      cas.publicKey,
    ];

    const tx = await projectProgram.methods
      .addInitialSignatories(projectBump,projectId, all, threshold, timeLimit)
      .accounts({
        baseAccount: projectPDA,
      })
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );
  });

  it("Create a proposal to add a new signatory", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("project"), Buffer.from(projectId.substring(0,18)), Buffer.from(projectId.substring(18,36))],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .addNewSignatoryProposal(projectBump, projectId, dan.publicKey)
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    assert(state.add.status, true);
    // assert(state.add.newSignatory, dan.publicKey)
  });

  it("Sign the add proposal", async () => {
    it("testing if it works or not", async () => {
      assert.equal(false, true);
    });

    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("project"), Buffer.from(projectId.substring(0,18)), Buffer.from(projectId.substring(18,36))],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .signProposal(projectBump, projectId, "add")
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    let state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.add.votes, 1);

    const tx1 = await projectProgram.methods
      .signProposal(projectBump,projectId, "add")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.add.votes, 0);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "add")
        .accounts({
          baseAccount: projectPDA,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();
      console.log("This should not get printed");
    } catch (error) {
      assert.equal(error.error.errorCode.code, "NoProposalCreated");
    }
  });
  
  it("initialize transfer program and deposit the amount for transfer", async() => {

    const [transferPDA, transferBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("transfer"), Buffer.from(transferId.substring(0,18)),Buffer.from(transferId.substring(18,36))],
      transferProgram.programId
    )

    const [projectPDA, projectBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("project"), Buffer.from(projectId.substring(0,18)), Buffer.from(projectId.substring(18,36))],
      projectProgram.programId
    )

    const [generalPDA, generalBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("general")],
      generalProgram.programId
    )

    const [projectPoolWalletPDA, projectPoolWalletBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("pool"), Buffer.from(transferId.substring(0,18)),Buffer.from(transferId.substring(18,36))],
      transferProgram.programId
    )

    let _casTokenAccountBefore = await spl.getAccount(provider.connection,casTokenAccount);

    const tx = await transferProgram.methods.initialize(transferId, generalBump, transferBump, transferAmount1, dan.publicKey).accounts({
      baseAccount: transferPDA,
      generalAccount: generalPDA,
      projectPoolWallet: projectPoolWalletPDA,
      tokenMint: USDCMint,
      authority: cas.publicKey,
      walletToWithdrawFrom: casTokenAccount,
      generalProgram: generalProgram.programId,
      systemProgram: anchor.web3.SystemProgram.programId,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY
    }).signers([cas]).rpc();

    const state = await transferProgram.account.transferParameter.fetch(transferPDA);
    assert.equal(state.amount, transferAmount1);

    let _casTokenAccountAfter = await spl.getAccount(provider.connection,casTokenAccount);
    let _poolWallet = await spl.getAccount(provider.connection, projectPoolWalletPDA);
    assert.equal(state.amount, _casTokenAccountBefore.amount - _casTokenAccountAfter.amount);
    assert.equal(state.amount, _poolWallet.amount);

  })

});
