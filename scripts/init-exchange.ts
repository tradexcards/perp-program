/**
 * Initialize the exchange state on-chain (one-time per program deployment).
 *
 * Reads config from .env file. Required env vars:
 *   - PROGRAM_ID: The deployed program address
 *   - USDC_MINT: USDC mint address for the network
 *   - SOLANA_RPC_URL: RPC endpoint
 *   - KEYPAIR_PATH: Path to admin keypair (default: ./keypair.json)
 *
 * Usage:
 *   bun run scripts/init-exchange.ts
 */
import {
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    TransactionInstruction,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// ── Load env ────────────────────────────────────────────────────────────────

function loadEnvFile(path: string): Record<string, string> {
    if (!existsSync(path)) return {};
    const content = readFileSync(path, "utf-8");
    const env: Record<string, string> = {};
    for (const line of content.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) continue;
        const eqIdx = trimmed.indexOf("=");
        if (eqIdx === -1) continue;
        const key = trimmed.slice(0, eqIdx).trim();
        const value = trimmed.slice(eqIdx + 1).trim();
        env[key] = value;
    }
    return env;
}

// Merge .env file with process.env (process.env takes precedence)
const fileEnv = loadEnvFile(resolve(ROOT, ".env"));
const env = { ...fileEnv, ...process.env };

// ── Constants (must match program/src/constants.rs) ─────────────────────────

const EXCHANGE_SEED = Buffer.from("exchange");
const VAULT_SEED = Buffer.from("vault");
const INSURANCE_SEED = Buffer.from("insurance");
const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const DISC_INITIALIZE_EXCHANGE = Buffer.from([0xe0, 0x69, 0x74, 0xa6, 0xe4, 0xcf, 0x60, 0x13]);

// ── Main ────────────────────────────────────────────────────────────────────

async function main() {
    // Validate required env vars
    const programIdStr = env.PROGRAM_ID;
    const usdcMintStr = env.USDC_MINT;
    const rpcUrl = env.SOLANA_RPC_URL;
    const keypairPath = env.KEYPAIR_PATH || resolve(ROOT, "keypair.json");

    if (!programIdStr) {
        console.error("ERROR: PROGRAM_ID env var is required");
        process.exit(1);
    }
    if (!usdcMintStr) {
        console.error("ERROR: USDC_MINT env var is required");
        process.exit(1);
    }
    if (!rpcUrl) {
        console.error("ERROR: SOLANA_RPC_URL env var is required");
        process.exit(1);
    }

    const programId = new PublicKey(programIdStr);
    const usdcMint = new PublicKey(usdcMintStr);
    const resolvedKeypairPath = keypairPath.startsWith("/") 
        ? keypairPath 
        : resolve(ROOT, keypairPath);

    console.log("");
    console.log("=== Initialize Exchange ===");
    console.log("");

    if (!existsSync(resolvedKeypairPath)) {
        console.error(`ERROR: Keypair not found at ${resolvedKeypairPath}`);
        process.exit(1);
    }

    const raw = JSON.parse(readFileSync(resolvedKeypairPath, "utf-8"));
    const admin = Keypair.fromSecretKey(new Uint8Array(raw));

    const [exchangeState] = PublicKey.findProgramAddressSync([EXCHANGE_SEED], programId);
    const [vault] = PublicKey.findProgramAddressSync([VAULT_SEED], programId);
    const [insuranceFund] = PublicKey.findProgramAddressSync([INSURANCE_SEED], programId);

    console.log("  Admin:          ", admin.publicKey.toString());
    console.log("  Program ID:     ", programId.toString());
    console.log("  RPC:            ", rpcUrl);
    console.log("  USDC Mint:      ", usdcMint.toString());
    console.log("  Exchange State: ", exchangeState.toString());
    console.log("  Vault:          ", vault.toString());
    console.log("  Insurance Fund: ", insuranceFund.toString());

    const connection = new Connection(rpcUrl, "confirmed");

    // Check if already initialized
    const existing = await connection.getAccountInfo(exchangeState);
    if (existing) {
        console.log("");
        console.log("  Exchange state already initialized.");
        console.log("    Owner:", existing.owner.toString());
        console.log("    Data:", existing.data.length, "bytes");
        return;
    }

    console.log("");
    console.log("  Sending InitializeExchange transaction...");

    const ix = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: exchangeState, isSigner: false, isWritable: true },
            { pubkey: usdcMint, isSigner: false, isWritable: false },
            { pubkey: vault, isSigner: false, isWritable: true },
            { pubkey: insuranceFund, isSigner: false, isWritable: true },
            { pubkey: admin.publicKey, isSigner: true, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: TOKEN_PROGRAM, isSigner: false, isWritable: false },
            { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        ],
        data: DISC_INITIALIZE_EXCHANGE,
    });

    const { blockhash } = await connection.getLatestBlockhash("confirmed");
    const message = new TransactionMessage({
        payerKey: admin.publicKey,
        recentBlockhash: blockhash,
        instructions: [ix],
    }).compileToV0Message();

    const tx = new VersionedTransaction(message);
    tx.sign([admin]);

    const sig = await connection.sendTransaction(tx, {
        skipPreflight: false,
        preflightCommitment: "confirmed",
    });

    const confirmation = await connection.confirmTransaction(sig, "confirmed");
    if (confirmation.value.err) {
        console.error("  Transaction failed:", confirmation.value.err);
        process.exit(1);
    }

    console.log("  Exchange initialized.");
    console.log("    Signature:", sig);
}

main().catch((e) => {
    console.error("Error:", e);
    process.exit(1);
});
