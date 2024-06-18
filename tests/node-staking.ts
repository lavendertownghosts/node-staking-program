import * as anchor from "@coral-xyz/anchor";
import {
  Program,
  Idl,
  BN,
  web3,
  
} from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddress,
  getAccount
} from "@solana/spl-token";
import { readFileSync } from "fs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { NodeStaking } from "../target/types/node_staking";

describe("node-staking", () => {
  // Configure the client to use the local cluster.
  // const poolAuthKeypair = web3.Keypair.fromSecretKey(
  //   Uint8Array.from(JSON.parse(readFileSync("/home/yb/.config/solana/poolAuth.json", "utf-8")))
  // )
  // const poolAuthorityProvider = new anchor.AnchorProvider(
  //   anchor.getProvider().connection,
  //   new anchor.Wallet(poolAuthKeypair),
  //   anchor.AnchorProvider.defaultOptions()
  // );
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.NodeStaking as Program<NodeStaking>;

  const TOKEN_METADATA_PROGRAM_ID = new web3.PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

  const [mint] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("mint")],
    program.programId
  )

  const [poolState] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pool_state")],
    program.programId
  )

  const [presaleState] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("presale_state")],
    program.programId
  )

  const [presaleVault] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("presale_vault")],
    program.programId
  )

  const [userStakeEntry] = web3.PublicKey.findProgramAddressSync(
    [provider.publicKey.toBuffer()],
    program.programId
  )

  const vaultAuthKey = new web3.PublicKey("6JvsMVc9rwY9AG63qsqrfoDcNPgRmx9JfMHMHaX7TRoS");

  it("Pool State is initialized!", async () => {
    const tokensPerNode = new BN(10);
    const rewardsPerNode = 20;
    const maxAllocation = 1000;
    const treasury_to_selling = 0.2;
    const treasuryVault = await getAssociatedTokenAddress(
      mint,
      provider.publicKey
    )
    const tx = await program?.methods.initializePool(tokensPerNode, rewardsPerNode, maxAllocation, treasury_to_selling)
      .accounts({
        treasuryVault
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("Selling vault is initialized!", async () => {
    const sellingVault = await getAssociatedTokenAddress(
      mint,
      poolState,
      true
    )
    const tx = await program?.methods.initializeSellingVault()
      .accounts({
        sellingVault
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  // it("Token is initialized!", async () => {
  //   const [metadata] = web3.PublicKey.findProgramAddressSync(
  //     [
  //       Buffer.from("metadata"),
  //       TOKEN_METADATA_PROGRAM_ID.toBuffer(),
  //       mint.toBuffer()
  //     ],
  //     TOKEN_METADATA_PROGRAM_ID
  //   )
  //   const tx = await program?.methods.initializeToken()
  //     .accounts({
  //       metadata
  //     })
  //     .rpc();
  //   console.log("Your transaction signature", tx);
  // });

  it("Token Mint!", async () => {
    // const sellingVault = await getAssociatedTokenAddress(
    //   mint,
    //   poolState,
    //   true
    // )
    const amount = new BN("50000000000")
    const treasuryVault = await getAssociatedTokenAddress(
      mint,
      provider.publicKey
    )
    const sellingVault = await getAssociatedTokenAddress(
      mint,
      poolState,
      true
    )
    const tx = await program?.methods.mintTokens(amount)
      .accounts({
        treasuryVault,
        sellingVault
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("Initilalize Presale", async () => {
    const pricePerNode = new BN(1);
    const maxAllocation = 1000;
    const presaleStartAt = new BN(Math.floor(new Date().getTime() / 1000))
    const presaleEndAt = new BN(Math.floor(new Date('2024.5.30').getTime() / 1000))
    const totalPresaleAmount = 10000;

    const tx = await program.methods.initializePresale(
      pricePerNode,
      maxAllocation,
      presaleStartAt,
      presaleEndAt,
      totalPresaleAmount
    ).rpc()

    console.log("initialize presale tx", tx)
  })

  it("Mint Nodes", async () => {
    const amount = 1000;

    const tx = await program.methods.mintNodes(amount).rpc()

    console.log("mint nodes transaction", tx)
  })

  it("Initialize User Stake Entry", async () => {
    const tx = await program.methods.initializeUserStake().rpc();

    console.log("Initialize user stake entry tx", tx)
  })

  it("Selling Nodes At Presale", async () => {
    const amount = 10;

    const tx = await program.methods.sellNodesAtPresale(amount)
      .accounts({
        presaleState
      })
      .rpc();

    console.log("Selling Nodes At Presale", tx)
  })

  it("withdraw cap", async () => {
    const poolAuthKeypair = web3.Keypair.fromSecretKey(
      bs58.decode(
        "EK8HWgbezLQxgkwZBb4WgQQGUrHvba8QG5pSrbPTfscwY5Hb7RPj1K6FhSDWmYfXNXvFuvtsYAkG1kv17MiNcRa"
      )
    )

    const airdropSig = await provider.connection.requestAirdrop(
      poolAuthKeypair.publicKey,
      web3.LAMPORTS_PER_SOL * 1
    )

    const latestBlockHash = await provider.connection.getLatestBlockhash();
    
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSig
    })  

    const tx = await program.methods.withdrawCap()
      .accounts({
        withdrawer: poolAuthKeypair.publicKey
      })
      .signers([poolAuthKeypair])
      .transaction()

    await web3.sendAndConfirmTransaction(
      program.provider.connection, 
      tx, 
      [poolAuthKeypair]
    )
  })

  it("create nodes!", async () => {
    const amount = 100;
    const userTokenAccount = await getAssociatedTokenAddress(
      mint,
      provider.publicKey
    )
    const treasuryVault = await getAssociatedTokenAddress(
      mint,
      provider.publicKey
    )
    const sellingVault = await getAssociatedTokenAddress(
      mint,
      poolState,
      true
    )
    const tx = await program.methods.createNodes(amount)
      .accounts({
        userTokenAccount,
        treasuryVault,
        sellingVault
      })
      .rpc()

    const [poolStateAddr] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool_state")],
      program.programId
    )

    const poolStateData = await program.account.poolState.fetch(poolStateAddr)

    console.log("nodes balance", poolStateData.totalNodes)

    console.log("create nodes transaction", tx)
  })

  it("Accounts data", async () => {
    const data = {}
    data.totalNodes = (await program.account.poolState.fetch(poolState)).totalNodes;
    const treasuryVault = await getAssociatedTokenAddress(
      mint,
      provider.publicKey
    )
    const sellingVault = await getAssociatedTokenAddress(
      mint,
      poolState,
      true
    )
    const treasuryAcc = await getAccount(provider.connection, treasuryVault)
    data.treasuryTokens = treasuryAcc.amount
    const sellingAcc = await getAccount(provider.connection, sellingVault)
    data.sellingTokens = sellingAcc.amount
    data.presaleVaultAmount = await provider.connection.getBalance(presaleVault)
    const userStakeEntryData = await program.account.userStakeEntry.fetch(userStakeEntry)
    data.userNodes = userStakeEntryData.stakedAmount
    data.claimableAmount = userStakeEntryData.claimableAmount.toString()
    const poolAuthKeypair = web3.Keypair.fromSecretKey(
      bs58.decode(
        "EK8HWgbezLQxgkwZBb4WgQQGUrHvba8QG5pSrbPTfscwY5Hb7RPj1K6FhSDWmYfXNXvFuvtsYAkG1kv17MiNcRa"
      )
    )
    data.vaultAuthBalance = await provider.connection.getBalance(poolAuthKeypair.publicKey)
    console.table(data)
  })
});


