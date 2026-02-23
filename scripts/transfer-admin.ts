/**
 * Transfer exchange admin to a new wallet (two-step process).
 * 
 * Step 1: Run with --initiate to set pending_admin
 * Step 2: Run with --accept from the NEW wallet to complete transfer
 * 
 * Required env vars:
 *   - PROGRAM_ID: The deployed program address
 *   - SOLANA_RPC_URL: RPC endpoint
 * 
 * Usage:
 *   # Step 1: Initiate transfer (signs with keypair.json as current admin)
 *   bun run scripts/transfer-admin.ts --initiate <NEW_ADMIN_PUBKEY>
 * 
 *   # Step 2: Accept transfer (signs with new admin keypair)
 *   bun run scripts/transfer-admin.ts --accept --keypair <NEW_KEYPAIR_PATH>
 */

import {
    Connection,
    Keypair,
    PublicKey,
    TransactionInstruction,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// Load env
function loadEnvFile(path: string): Record<string, string> {
    if (!existsSync(path)) return {};
    const content = readFileSync(path, "utf-8");
    const env: Record<string, string> = {};
    for (const line of content.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) continue;
        const eqIdx = trimmed.indexOf("=");
        if (eqIdx === -1) continue;
        env[trimmed.slice(0, eqIdx).trim()] = trimmed.slice(eqIdx + 1).trim();
    }
    return env;
}

const fileEnv = loadEnvFile(resolve(ROOT, ".env"));
const env = { ...fileEnv, ...process.env };

// Program constants
const EXCHANGE_SEED = Buffer.from("exchange");

// Discriminators
const DISC_TRANSFER_ADMIN = Buffer.from([0x2a, 0xf2, 0x42, 0x6a, 0xe4, 0x0a, 0x6f, 0x9c]);
const DISC_ACCEPT_ADMIN = Buffer.from([0x70, 0x2a, 0x2d, 0x5a, 0x74, 0xb5, 0x0d, 0xaa]);

function loadKeypair(path: string): Keypair {
    const resolved = path.startsWith("/") ? path : resolve(ROOT, path);
    const raw = JSON.parse(readFileSync(resolved, "utf-8"));
    return Keypair.fromSecretKey(new Uint8Array(raw));
}

async function sendTx(connection: Connection, keypair: Keypair, ix: TransactionInstruction): Promise<string> {
    const { blockhash } = await connection.getLatestBlockhash();
    const message = new TransactionMessage({
        payerKey: keypair.publicKey,
        recentBlockhash: blockhash,
        instructions: [ix],
    }).compileToV0Message();

    const tx = new VersionedTransaction(message);
    tx.sign([keypair]);

    const sig = await connection.sendTransaction(tx, { skipPreflight: false });
    console.log(`TX sent: ${sig}`);
    
    await connection.confirmTransaction(sig, "confirmed");
    console.log("Confirmed!");
    
    return sig;
}

async function initiateTransfer(connection: Connection, programId: PublicKey, newAdminPubkey: PublicKey) {
    console.log("\n=== Initiating Admin Transfer ===\n");
    
    // Load current admin keypair
    const currentAdmin = loadKeypair("keypair.json");
    console.log("Current admin:", currentAdmin.publicKey.toBase58());
    console.log("New admin:    ", newAdminPubkey.toBase58());
    
    const [exchangePDA] = PublicKey.findProgramAddressSync([EXCHANGE_SEED], programId);
    console.log("Exchange PDA: ", exchangePDA.toBase58());
    
    // Build transfer_admin instruction
    // Data: discriminator (8) + new_admin pubkey (32)
    const data = Buffer.alloc(40);
    DISC_TRANSFER_ADMIN.copy(data, 0);
    newAdminPubkey.toBuffer().copy(data, 8);
    
    const ix = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: exchangePDA, isSigner: false, isWritable: true },
            { pubkey: currentAdmin.publicKey, isSigner: true, isWritable: false },
        ],
        data,
    });
    
    console.log("\nSending transfer_admin transaction...");
    const sig = await sendTx(connection, currentAdmin, ix);
    
    console.log("\n✓ Admin transfer initiated!");
    console.log(`  Signature: ${sig}`);
    console.log(`\nNext step: Run with --accept from the new admin wallet to complete the transfer.`);
}

async function acceptTransfer(connection: Connection, programId: PublicKey, keypairPath: string) {
    console.log("\n=== Accepting Admin Transfer ===\n");
    
    // Load new admin keypair
    const newAdmin = loadKeypair(keypairPath);
    console.log("New admin:", newAdmin.publicKey.toBase58());
    
    const [exchangePDA] = PublicKey.findProgramAddressSync([EXCHANGE_SEED], programId);
    console.log("Exchange PDA:", exchangePDA.toBase58());
    
    // Build accept_admin instruction
    // Data: discriminator only (8)
    const ix = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: exchangePDA, isSigner: false, isWritable: true },
            { pubkey: newAdmin.publicKey, isSigner: true, isWritable: false },
        ],
        data: DISC_ACCEPT_ADMIN,
    });
    
    console.log("\nSending accept_admin transaction...");
    const sig = await sendTx(connection, newAdmin, ix);
    
    console.log("\n✓ Admin transfer complete!");
    console.log(`  Signature: ${sig}`);
    console.log(`  New admin: ${newAdmin.publicKey.toBase58()}`);
}

async function main() {
    const args = process.argv.slice(2);
    
    const programIdStr = env.PROGRAM_ID;
    const rpcUrl = env.SOLANA_RPC_URL;
    
    if (!programIdStr) {
        console.error("ERROR: PROGRAM_ID env var is required");
        process.exit(1);
    }
    if (!rpcUrl) {
        console.error("ERROR: SOLANA_RPC_URL env var is required");
        process.exit(1);
    }
    
    const programId = new PublicKey(programIdStr);
    const connection = new Connection(rpcUrl, "confirmed");
    
    if (args.includes("--initiate")) {
        const pubkeyIdx = args.indexOf("--initiate") + 1;
        if (!args[pubkeyIdx]) {
            console.error("ERROR: Must provide new admin pubkey after --initiate");
            console.error("Usage: bun run scripts/transfer-admin.ts --initiate <NEW_ADMIN_PUBKEY>");
            process.exit(1);
        }
        const newAdminPubkey = new PublicKey(args[pubkeyIdx]);
        await initiateTransfer(connection, programId, newAdminPubkey);
        
    } else if (args.includes("--accept")) {
        const keypairIdx = args.indexOf("--keypair");
        const keypairPath = keypairIdx !== -1 ? args[keypairIdx + 1] : "keypair.json";
        await acceptTransfer(connection, programId, keypairPath);
        
    } else {
        console.log("Transfer Exchange Admin");
        console.log("");
        console.log("Usage:");
        console.log("  # Step 1: Initiate transfer (uses keypair.json as current admin)");
        console.log("  bun run scripts/transfer-admin.ts --initiate <NEW_ADMIN_PUBKEY>");
        console.log("");
        console.log("  # Step 2: Accept transfer (uses new keypair)");
        console.log("  bun run scripts/transfer-admin.ts --accept --keypair <NEW_KEYPAIR_PATH>");
    }
}

main().catch((e) => {
    console.error("Error:", e.message || e);
    process.exit(1);
});
