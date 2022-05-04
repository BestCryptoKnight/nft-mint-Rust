import * as anchor from '@project-serum/anchor';
import { NftMaker } from '../target/types/nft_maker';

import { Program, BN, } from "@project-serum/anchor";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram, 
  SYSVAR_RENT_PUBKEY, 
  LAMPORTS_PER_SOL,
  AccountMeta,
  TransactionInstruction,
  Transaction,

} from "@solana/web3.js";

import assert from 'assert';

const {
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID, 
  Token
} = require("@solana/spl-token");

import { struct, u8, u32 } from '@solana/buffer-layout';



describe('nft-maker', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.NftMaker as Program<NftMaker>;

  const seed = "nft-maker";
  const recipient = Keypair.generate().publicKey;
  const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey(
    'metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s',
  );

  it('Is initialized!', async () => {

    const [configKey, configNonce] = await PublicKey.findProgramAddress(
      [Buffer.from(seed)],
      program.programId
    );

    const [vaultkey, vaultNonce] = await PublicKey.findProgramAddress(
      [configKey.toBuffer()],
      program.programId
    );

    console.log("configKey: ", configKey.toString());
    console.log("vaultkey: ", vaultkey.toString());

    const configAccountInfo = await provider.connection.getAccountInfo(configKey);
    if (configAccountInfo) {
      console.log("program had been initialized!");
    } else {
      const amount = new BN(LAMPORTS_PER_SOL);
      const tx = await program.rpc.initialize(
        configNonce,
        vaultNonce,
        provider.wallet.publicKey,
        amount,
        {
          accounts: {
            signer: provider.wallet.publicKey,
            payerVault: vaultkey,
            nftMintSettings: configKey,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY
          },
          signers: [provider.wallet.payer],
  
        });
        const vaultBanlance = await provider.connection.getBalance(vaultkey);
        console.log("tx: ", tx);
        assert.equal(vaultBanlance, LAMPORTS_PER_SOL);
    }
    
  });

 
  it('mint one NFT!', async () => {

    const listener = program.addEventListener("MintEvent", (event, slot) => {
      console.log("slot: ", slot);
      console.log("event status: ", event.status);
      console.log("event mint: ", event.mint);
      console.log("event recipient: ", event.recipient);
      console.log("event nft count: ", event.nftCount);
      program.removeEventListener(listener);
    });

    const [mintKey, mintNonce] = await PublicKey.findProgramAddress(
      [Buffer.from("7876875575")],
      program.programId
    );

    const assTokenKey = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      mintKey,
      recipient
    );

    const [configKey, ] = await PublicKey.findProgramAddress(
      [Buffer.from(seed)],
      program.programId
    );

    const [vaultkey, nonce] = await PublicKey.findProgramAddress(
      [configKey.toBuffer()],
      program.programId
    );
  
    const [metadatakey, _] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from('metadata'),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mintKey.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID,
    );
  
    const [masterkey, __] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from('metadata'),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mintKey.toBuffer(),
        Buffer.from('edition'),
      ],
      TOKEN_METADATA_PROGRAM_ID,
    );

    console.log("mintKey: ", mintKey.toString());
    console.log("configKey: ", configKey.toString());
    console.log("payerVault: ", vaultkey.toString());

    console.log("recipient: ", recipient.toString());
    console.log("assTokenKey: ", assTokenKey.toString());

    console.log("metadatakey: ", metadatakey.toString());
    console.log("masterkey: ", masterkey.toString());

    
    enum TokenInstruction {
      RequestUnits = 0,
      RequestHeapFrame = 1,
    }
    interface RequestUnitsInstructionData {
      instruction: TokenInstruction.RequestUnits;
      units: number;
      additional_fee: number;
    }

    const burnCheckedInstructionData = struct<RequestUnitsInstructionData>([
      u8('instruction'),
      u32('units'),
      u32('additional_fee'),
    ]);

    function createRequestUnitsInstruction(
      units: number,
      additional_fee: number
    ): TransactionInstruction {
      const keys: AccountMeta[] = [];
      const data = Buffer.alloc(burnCheckedInstructionData.span);
      burnCheckedInstructionData.encode(
          {
              instruction: TokenInstruction.RequestUnits,
              units,
              additional_fee,
          },
          data
      );
      const programId = new PublicKey('ComputeBudget111111111111111111111111111111');

      return new TransactionInstruction({
          programId,
          keys,
          data
      });

    }

    
    const mintNftIns = program.instruction.mintingNft(
      "test-NFT",
      "7876875575",
      "https://arweave.net/sCuT4ASiUgq7JxgU_3aoq0xJLpwH2Z1z2R2_xwPM8uc",
      1000,
      false,
      mintNonce,
      {
        accounts: {
          signer: provider.wallet.publicKey,
          recipient: recipient,
          recipientToken: assTokenKey,
          payerVault: vaultkey,
          nftMintSettings: configKey,
          mint: mintKey,
          metadata: metadatakey,
          masteredition: masterkey,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY
        },
        signers: [provider.wallet.payer],
        
      });

    const requestUnitsIns = createRequestUnitsInstruction(250000, 0);
    const trans = new Transaction().add(
      requestUnitsIns,
    )
    .add(
      mintNftIns,
    );
    const tx = await provider.send(trans);
    console.log("tx: ", tx);

  });


});