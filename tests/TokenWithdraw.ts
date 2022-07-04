import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { TokenWithdraw } from "../target/types/token_withdraw";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { Buffer } from "buffer";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
  createMint,
  mintTo
} from "@solana/spl-token"; 
import { assert } from "chai";

describe("anchor_withraw", async() => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);
  const program = anchor.workspace.TokenWithdraw as Program<TokenWithdraw>;
  
  const start_time = Math.floor(Date.now()/1000);
  const amount = (5* LAMPORTS_PER_SOL).toString();
  const token_amount = (500).toString();

  const ESCROW_PDA_SEED = "escrow_seed";
  const TOKEN_ESCROW_PDA_SEED = "token_escrow_seed";

  const vault = anchor.web3.Keypair.generate();
  const sender_account = anchor.web3.Keypair.generate();
  const receiver_account = anchor.web3.Keypair.generate();
  let escrow_account: PublicKey = null;
  let token_escrow: PublicKey = null;

  const [_escrow_account, _bump] = await PublicKey.findProgramAddress([
    Buffer.from(anchor.utils.bytes.utf8.encode(ESCROW_PDA_SEED)), 
    sender_account.publicKey.toBuffer()
  ],
  program.programId);

  const [_token_escrow_account, _token_bump] = await PublicKey.findProgramAddress([
    Buffer.from(anchor.utils.bytes.utf8.encode(TOKEN_ESCROW_PDA_SEED)), 
    sender_account.publicKey.toBuffer()
  ],
  program.programId);
  escrow_account = _escrow_account;     // pda to store info about native sol
  token_escrow = _token_escrow_account;  // pda to store info about tokens

  let mint = null;
  let vault_ata = null;
  let receiver_ata = null;
  let sender_ata = null;


  const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: PublicKey = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',);
  const SYSTEMRENT = "SysvarRent111111111111111111111111111111111";
  
  it("Initiallize Native Sol", async () => {
    //funding sender account
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(sender_account.publicKey, 1000000000000),
      "processed"
    );

    let balance = await provider.connection.getBalance(sender_account.publicKey);
    console.log(`sender_balance is ${balance / LAMPORTS_PER_SOL} SOL`);
    
    let vault_balance = await provider.connection.getBalance(vault.publicKey);
    console.log("vault_balance before initiallize =", vault_balance/LAMPORTS_PER_SOL);

    await program.rpc.initializeNativeSol(
      new anchor.BN(start_time),
      new anchor.BN(amount),
      {
      accounts:{
        escrowAccount: escrow_account,
        senderAccount: sender_account.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        receiverAccount: receiver_account.publicKey,
        vault: vault.publicKey
        },
      signers: [sender_account]
      }
    );

    let escrow_data = await program.account.escrowNative.fetch(escrow_account);
    console.log("amount: ", escrow_data.amount.toNumber()/LAMPORTS_PER_SOL);
    assert.ok(escrow_data.amount.toNumber(), amount.toString());
    
    vault_balance = await provider.connection.getBalance(vault.publicKey);
    assert.ok(escrow_data.amount.toNumber(),vault_balance.toString());
    console.log("vault_balance after init =", vault_balance/LAMPORTS_PER_SOL);
  });

  it("Withdraw Native Sol", async () => {
    //wait 3 sec because acc to sc user can withdraw only after 2 sec of `escrow_account.start_time`
    const delay = ms => new Promise(res => setTimeout(res, ms));
    await delay(3000);

    await program.rpc.withdrawNativeSol(
      new anchor.BN(amount),
      {
      accounts:{
        escrowAccount: escrow_account,
        senderAccount: sender_account.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        receiverAccount: receiver_account.publicKey,
        vault: vault.publicKey
        },
      signers: [vault]
      }
    );


    let escrow_data = await program.account.escrowNative.fetch(escrow_account);
    assert.ok(escrow_data.amount.toNumber(), amount.toString());
    
    let vault_balance = await provider.connection.getBalance(vault.publicKey);
    assert.ok(escrow_data.amount.toNumber(),vault_balance.toString());
    console.log("vault_balance after withdraw =", vault_balance/LAMPORTS_PER_SOL);
    
    let receiver_balance = await provider.connection.getBalance(receiver_account.publicKey);
    assert.ok(escrow_data.amount.toNumber(),receiver_balance.toString());
    console.log("receiver_balance after withdraw =", receiver_balance/LAMPORTS_PER_SOL);
    console.log(">>>>>>>>>>>>>>>>>>>>>Testing Native Completed<<<<<<<<<<<<<<<<<<")
  });

  it("token init" ,async () => {
    /* =========== Spl token test =================== */

    mint = await createMint(
      provider.connection,
      sender_account,
      sender_account.publicKey,
      null,
      0,
    );
    
    vault_ata = getAssociatedTokenAddress(mint, vault.publicKey, false);
    receiver_ata = getAssociatedTokenAddress(mint, receiver_account.publicKey, false);

    console.log("mint key", mint.toBase58());
    sender_ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      sender_account,
      mint,
      sender_account.publicKey
    );
    
    console.log("sender ATA", sender_ata.address.toBase58());

    //minting 100 new tokens to the token address we just created
    provider.connection,
    await mintTo(
      provider.connection,
      sender_account, 
      mint, 
      sender_ata.address,
      sender_account,
      Number(token_amount)
      );

    let sender_ata_token = await provider.connection.getTokenAccountBalance(sender_ata.address);
    console.log("Total minted token to sender ATA : ", Number(sender_ata_token.value.amount));
    
    await program.rpc.intializeFungibleToken(
      new anchor.BN(start_time),
      new anchor.BN(token_amount),
      {
      accounts:{
        escrowAccount: token_escrow,
        senderAssociatedInfo: sender_ata.address,
        vaultAssociatedInfo: (await vault_ata).toBase58(),
        senderAccount: sender_account.publicKey,
        vault: vault.publicKey,
        receiverAccount: receiver_account.publicKey,
        mint:mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram:SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
        rent:SYSTEMRENT
        },
      signers: [sender_account]
      }
    );
    
    let escrow_data_token = await program.account.escrowFungibleToken.fetch(token_escrow);
    console.log("amount in escrow: ", escrow_data_token.amountToken.toNumber());
    assert.ok(escrow_data_token.amountToken.toNumber(), token_amount.toString());
    
    
  });

  it("withdraw token", async() =>{

    let vault_ata_token_value = await provider.connection.getTokenAccountBalance(await vault_ata);
    console.log("vault token value before transfer : ", vault_ata_token_value.value.amount);

    //vault account needs sol because it needs to create its ATA
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(receiver_account.publicKey, 10000000000),
      "processed"
    );

    await program.rpc.withdrawFungibleToken(
      new anchor.BN(token_amount),
      {
      accounts:{
        escrowAccount: token_escrow,
        receiverAssociatedInfo: (await receiver_ata).toBase58(),
        vaultAssociatedInfo: (await vault_ata).toBase58(),
        senderAccount: sender_account.publicKey,
        vault: vault.publicKey,
        receiverAccount: receiver_account.publicKey,
        mint:mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram:SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
        rent:SYSTEMRENT
        },
      signers: [receiver_account, vault]
      }
    );

    let receiver_ata_token_value = await provider.connection.getTokenAccountBalance(await receiver_ata);
    console.log("Receiver token value after transfer : ", receiver_ata_token_value.value.amount);

    vault_ata_token_value = await provider.connection.getTokenAccountBalance(await vault_ata);
    console.log("Vault token value after transfer : ", vault_ata_token_value.value.amount);

    console.log(">>>>>>>>>>>>>>>>>>>>>Testing TOKEN Completed<<<<<<<<<<<<<<<<<<");

  });
})
