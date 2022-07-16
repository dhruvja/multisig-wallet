import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Project } from "../target/types/project";
import { General } from "../target/types/general";
import { Transfer } from "../target/types/transfer";
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
  let extra: anchor.web3.Keypair;
  let admin: anchor.web3.Keypair;

  let USDCMint: anchor.web3.PublicKey; // token which would be staked
  let casTokenAccount: anchor.web3.PublicKey; // cas token account
  let adminTokenAccount: anchor.web3.PublicKey; // admin token account

  let initialMintAmount = 100000000;

  alice = anchor.web3.Keypair.generate();
  bob = anchor.web3.Keypair.generate();
  cas = anchor.web3.Keypair.generate();
  dan = anchor.web3.Keypair.generate();
  extra = anchor.web3.Keypair.generate();
  admin = anchor.web3.Keypair.generate(); // Admin

  const threshold = 2;
  const newThreshold = 6;
  const fallBackThreshold = 2;
  const timeLimit = 100 * 60 * 60 * 24; // 1 day
  const newTimeLimit = 60 * 60 * 24 * 2; // 2 days
  const percentTransfer = 2;

  const transferAmount1 = 1000;
  const withdrawAmount1 = 500;

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

    adminTokenAccount = await spl.createAccount(
      provider.connection,
      admin,
      USDCMint,
      admin.publicKey
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

    const state = await generalProgram.account.generalParameter.fetch(
      generalPDA
    );
    assert.equal(state.tokenMint.toBase58(), USDCMint.toBase58());

    try {
      await generalProgram.methods
        .changeMint(generalBump)
        .accounts({
          baseAccount: generalPDA,
          tokenMint: USDCMint,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();

      throw "Error occured, invalid authority while initializing general program";
    } catch (error) {
      assert.equal(error.error.errorCode.number, 2001);
    }
  });

  let projectId = uuidv4();
  const transferId = uuidv4();

  console.log(projectId);

  it("initializes project program", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [projectPoolPDA, projectPoolBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("pool"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [generalPDA, generalBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("general")],
        generalProgram.programId
      );

    const tx = await projectProgram.methods
      .initialize(projectId, percentTransfer)
      .accounts({
        baseAccount: projectPDA,
        projectPoolAccount: projectPoolPDA,
        tokenMint: USDCMint,
        authority: alice.publicKey,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([alice])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

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

    assert.equal(state.threshold, 1);
  });

  it("Cannot transfer when the threshold or the signatory is only 1", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [projectPoolPDA, projectPoolBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("pool"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [generalPDA, generalBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("general")],
        generalProgram.programId
      );

    const tx = await projectProgram.methods
      .transferAmountProposal(
        projectBump,
        projectId,
        withdrawAmount1,
        casTokenAccount
      )
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    assert(state.transferAmount.status, true);

    try {
      const tx = await projectProgram.methods
        .signTransfer(generalBump, projectBump, projectPoolBump, projectId)
        .accounts({
          baseAccount: projectPDA,
          generalAccount: generalPDA,
          projectPoolAccount: projectPoolPDA,
          tokenMint: USDCMint,
          authority: alice.publicKey,
          walletToWithdrawFrom: casTokenAccount,
          generalProgram: generalProgram.programId,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([alice])
        .rpc();
    } catch (error) {
      assert.equal(
        error.error.errorCode.code,
        "CannotTransferDueToLowThreshold"
      );
    }
  });

  it("Initialze the signatories for the project", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const all = [admin.publicKey, bob.publicKey, cas.publicKey];

    const tx = await projectProgram.methods
      .addInitialSignatories(projectBump, projectId, all, threshold, timeLimit)
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    try {
      const tx = await projectProgram.methods
        .addInitialSignatories(
          projectBump,
          projectId,
          all,
          threshold,
          timeLimit
        )
        .accounts({
          baseAccount: projectPDA,
          authority: bob.publicKey,
        })
        .signers([bob])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "InvalidSigner");
    }

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    const allSignatories = [
      alice.publicKey,
      admin.publicKey,
      bob.publicKey,
      cas.publicKey,
    ];

    for (let i = 0; i < state.signatories.length; i++) {
      if (state.signatories[i].key.toBase58() != allSignatories[i].toBase58())
        throw "All signatories are not added";
    }
  });

  it("Create a proposal to add a new signatory", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const newSigs = [
      dan.publicKey,
      extra.publicKey
    ];

    const tx = await projectProgram.methods
      .addNewSignatoryProposal(projectBump, projectId, newSigs)
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
  });

  it("Sign the add proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
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

    try {
      await projectProgram.methods
        .signProposal(projectBump, projectId, "add")
        .accounts({
          baseAccount: projectPDA,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();
      throw "repeated signature";
    } catch (error) {
      assert.equal(error.error.errorCode.code, "RepeatedSignature");
    }

    const tx1 = await projectProgram.methods
      .signProposal(projectBump, projectId, "add")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    const lastIndex = state.signatories.length;

    assert.equal(state.add.votes, 0);
    assert.equal(
      state.signatories[lastIndex - 1].key.toBase58(),
      extra.publicKey.toBase58()
    );
    assert.equal(
      state.signatories[lastIndex - 2].key.toBase58(),
      dan.publicKey.toBase58()
    );
    assert.equal(state.add.status, false);

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

  it("Create a delete proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .removeSignatoryProposal(projectBump, projectId, dan.publicKey)
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    assert(state.delete.status, true);
  });

  it("signs the delete proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .signProposal(projectBump, projectId, "delete")
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    let state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.delete.votes, 1);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "delete")
        .accounts({
          baseAccount: projectPDA,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "RepeatedSignature");
    }

    const tx1 = await projectProgram.methods
      .signProposal(projectBump, projectId, "delete")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.delete.votes, 0);

    for (let i = 0; i < state.signatories.length; i++) {
      if (state.signatories[i].key.toBase58() == dan.publicKey.toBase58()) {
        throw "Signatory has not been deleted";
      }
    }

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "delete")
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

  it("Create a change time out proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .changeTimeLimitProposal(projectBump, projectId, newTimeLimit)
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    assert.equal(state.changeTimeLimit.status, true);
    assert.equal(state.changeTimeLimit.newTimeLimit, newTimeLimit);
  });

  it("signs the change time out proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .signProposal(projectBump, projectId, "change time limit")
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    let state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.changeTimeLimit.votes, 1);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "change time limit")
        .accounts({
          baseAccount: projectPDA,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "RepeatedSignature");
    }

    const tx1 = await projectProgram.methods
      .signProposal(projectBump, projectId, "change time limit")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.changeTimeLimit.votes, 0);
    assert.equal(state.timeLimit, newTimeLimit);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "change time limit")
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

  it("Create a change threshold proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const currentTimestamp = new Date().getTime() / 1000;

    const tx = await projectProgram.methods
      .changeThresholdProposal(
        projectBump,
        projectId,
        newThreshold,
        currentTimestamp
      )
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const state = await projectProgram.account.projectParameter.fetch(
      projectPDA
    );

    assert(state.changeThreshold.status, true);
  });

  it("signs the change threshold proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const tx = await projectProgram.methods
      .signProposal(projectBump, projectId, "change threshold")
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    let state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.changeThreshold.votes, 1);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "change threshold")
        .accounts({
          baseAccount: projectPDA,
          authority: alice.publicKey,
        })
        .signers([alice])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "RepeatedSignature");
    }

    const tx1 = await projectProgram.methods
      .signProposal(projectBump, projectId, "change threshold")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.changeThreshold.votes, 0);
    assert.equal(state.threshold, newThreshold);

    try {
      const tx = await projectProgram.methods
        .signProposal(projectBump, projectId, "change threshold")
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

  it("Deposit tokens to project pool wallet", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [projectPoolPDA, projectPoolBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("pool"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [generalPDA, generalBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("general")],
        generalProgram.programId
      );

    const casTokenAccountBefore = await spl.getAccount(
      provider.connection,
      casTokenAccount
    );

    const tx = await projectProgram.methods
      .depositFunds(
        projectId,
        projectBump,
        projectPoolBump,
        generalBump,
        transferAmount1
      )
      .accounts({
        baseAccount: projectPDA,
        generalAccount: generalPDA,
        projectPoolAccount: projectPoolPDA,
        tokenMint: USDCMint,
        authority: cas.publicKey,
        walletToWithdrawFrom: casTokenAccount,
        adminTokenWallet: adminTokenAccount,
        generalProgram: generalProgram.programId,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([cas])
      .rpc();

    const casTokenAccountAfter = await spl.getAccount(
      provider.connection,
      casTokenAccount
    );

    const _adminTokenAccount = await spl.getAccount(
      provider.connection,
      adminTokenAccount
    );

    assert.equal(
      casTokenAccountBefore.amount - casTokenAccountAfter.amount,
      transferAmount1
    );

    assert.equal(
      _adminTokenAccount.amount,
      (transferAmount1 * percentTransfer) / 100
    );
  });

  it("Create a transfer proposal", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    try {
      const tx = await projectProgram.methods
        .transferAmountProposal(
          projectBump,
          projectId,
          withdrawAmount1,
          casTokenAccount
        )
        .accounts({
          baseAccount: projectPDA,
          authority: admin.publicKey,
        })
        .signers([admin])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "ProposalInProgress");
    }
  });

  it("transfer the funds after signing", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [projectPoolPDA, projectPoolBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("pool"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const [generalPDA, generalBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("general")],
        generalProgram.programId
      );

    const casTokenAccountBefore = await spl.getAccount(
      provider.connection,
      casTokenAccount
    );

    // console.log(casTokenAccountBefore.amount);

    const tx = await projectProgram.methods
      .signTransfer(generalBump, projectBump, projectPoolBump, projectId)
      .accounts({
        baseAccount: projectPDA,
        generalAccount: generalPDA,
        projectPoolAccount: projectPoolPDA,
        tokenMint: USDCMint,
        authority: cas.publicKey,
        walletToWithdrawFrom: casTokenAccount,
        generalProgram: generalProgram.programId,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([cas])
      .rpc();

    const casTokenAccountAfter = await spl.getAccount(
      provider.connection,
      casTokenAccount
    );

    // console.log(casTokenAccountAfter.amount);
  });

  it("Reduces the number of approvals after 90 days", async () => {
    const [projectPDA, projectBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("project"),
          Buffer.from(projectId.substring(0, 18)),
          Buffer.from(projectId.substring(18, 36)),
        ],
        projectProgram.programId
      );

    const timestampAfter90Days = new Date(2022, 9, 15).getTime() / 1000;
    const currentTimestamp = new Date().getTime() / 1000;

    let days = 60 * 60 * 24;

    let numberOfDays = (timestampAfter90Days - currentTimestamp) / days;
    let numberOfMonths = (numberOfDays - 90) / 30 + 1;

    const tx = await projectProgram.methods
      .changeThresholdProposal(
        projectBump,
        projectId,
        fallBackThreshold,
        timestampAfter90Days
      )
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    let state = await projectProgram.account.projectParameter.fetch(projectPDA);
    const newApproval = state.threshold - Math.round(numberOfMonths);

    assert.equal(state.approval, newApproval);

    const timestampAfter100Days = new Date(2022, 9, 25).getTime() / 1000;

    try {
      await projectProgram.methods
        .changeThresholdProposal(
          projectBump,
          projectId,
          fallBackThreshold,
          timestampAfter100Days
        )
        .accounts({
          baseAccount: projectPDA,
          authority: admin.publicKey,
        })
        .signers([admin])
        .rpc();
    } catch (error) {
      assert.equal(error.error.errorCode.code, "MinimumTimeNotPassed");
    }

    const timestampAfter150Days = new Date(2022, 11, 15).getTime() / 1000;
    numberOfDays = (timestampAfter150Days - timestampAfter90Days) / days;
    numberOfMonths = numberOfDays / 30;

    const tx1 = await projectProgram.methods
      .changeThresholdProposal(
        projectBump,
        projectId,
        fallBackThreshold,
        timestampAfter150Days
      )
      .accounts({
        baseAccount: projectPDA,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.approval, newApproval - Math.round(numberOfMonths));

    await projectProgram.methods
      .signProposal(projectBump, projectId, "change threshold")
      .accounts({
        baseAccount: projectPDA,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc();

    await projectProgram.methods
      .signProposal(projectBump, projectId, "change threshold")
      .accounts({
        baseAccount: projectPDA,
        authority: bob.publicKey,
      })
      .signers([bob])
      .rpc();

    await projectProgram.methods
      .signProposal(projectBump, projectId, "change threshold")
      .accounts({
        baseAccount: projectPDA,
        authority: cas.publicKey,
      })
      .signers([cas])
      .rpc();

    state = await projectProgram.account.projectParameter.fetch(projectPDA);
    assert.equal(state.threshold, fallBackThreshold);
    assert.equal(state.approval, fallBackThreshold);
    assert.equal(state.changeThreshold.status, false);
    assert.equal(state.changeThreshold.votes, 0);
    assert.equal(state.changeThreshold.timestamp, 0);
    assert.equal(state.lastReducedThreshold, 0);
  });
});
