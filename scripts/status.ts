/**
 * Check the on-chain status of the deployed program.
 *
 * Reads config from .env file. Required env vars:
 *   - PROGRAM_ID: The deployed program address
 *   - SOLANA_RPC_URL: RPC endpoint
 *   - KEYPAIR_PATH: Path to admin keypair (optional, for balance check)
 *
 * Usage:
 *   bun run scripts/status.ts
 */
import {
    Connection,
    PublicKey,
    Keypair,
} from "@solana/web3.js";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// Seeds (must match program constants)
const EXCHANGE_SEED = Buffer.from("exchange");
const ORACLE_SEED = Buffer.from("oracle");
const MARKET_SEED = Buffer.from("market");
const VAULT_SEED = Buffer.from("vault");
const INSURANCE_SEED = Buffer.from("insurance");

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

function marketPDA(programId: PublicKey, productId: number): PublicKey {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(productId, 0);
    return PublicKey.findProgramAddressSync([MARKET_SEED, buf], programId)[0];
}

async function main() {
    const programIdStr = env.PROGRAM_ID;
    const rpcUrl = env.SOLANA_RPC_URL;
    const keypairPath = env.KEYPAIR_PATH || resolve(ROOT, "keypair.json");

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

    console.log("══════════════════════════════════════════════");
    console.log("  On-Chain Status");
    console.log("══════════════════════════════════════════════");
    console.log("");
    console.log(`  Network:    ${rpcUrl}`);
    console.log(`  Program ID: ${programId.toString()}`);
    console.log("");

    // ── Program ─────────────────────────────────────────────────────────────
    const programInfo = await connection.getAccountInfo(programId);
    if (!programInfo) {
        console.log("  Program:    NOT FOUND");
        return;
    }
    console.log(`  Program:    deployed (${programInfo.data.length} bytes)`);

    // ── Payer balance ───────────────────────────────────────────────────────
    const resolvedKeypairPath = keypairPath.startsWith("/") 
        ? keypairPath 
        : resolve(ROOT, keypairPath);
    
    if (existsSync(resolvedKeypairPath)) {
        const raw = JSON.parse(readFileSync(resolvedKeypairPath, "utf-8"));
        const kp = Keypair.fromSecretKey(new Uint8Array(raw));
        const balance = await connection.getBalance(kp.publicKey);
        console.log(`  Authority:  ${kp.publicKey.toString()} (${(balance / 1e9).toFixed(4)} SOL)`);
    }

    // ── Exchange State ──────────────────────────────────────────────────────
    const [exchangePda] = PublicKey.findProgramAddressSync([EXCHANGE_SEED], programId);
    const exchangeInfo = await connection.getAccountInfo(exchangePda);
    console.log(`  Exchange:   ${exchangeInfo ? `initialized (${exchangeInfo.data.length} bytes)` : "NOT INITIALIZED"}`);

    // ── Oracle State ────────────────────────────────────────────────────────
    const [oraclePda] = PublicKey.findProgramAddressSync([ORACLE_SEED], programId);
    const oracleInfo = await connection.getAccountInfo(oraclePda);
    console.log(`  Oracle:     ${oracleInfo ? `initialized (${oracleInfo.data.length} bytes)` : "NOT INITIALIZED"}`);

    // ── Vault ───────────────────────────────────────────────────────────────
    const [vaultPda] = PublicKey.findProgramAddressSync([VAULT_SEED], programId);
    const vaultInfo = await connection.getAccountInfo(vaultPda);
    console.log(`  Vault:      ${vaultPda.toString()} ${vaultInfo ? "(exists)" : "(not found)"}`);

    // ── Insurance Fund ──────────────────────────────────────────────────────
    const [insurancePda] = PublicKey.findProgramAddressSync([INSURANCE_SEED], programId);
    const insuranceInfo = await connection.getAccountInfo(insurancePda);
    console.log(`  Insurance:  ${insurancePda.toString()} ${insuranceInfo ? "(exists)" : "(not found)"}`);

    // ── Sample Markets ──────────────────────────────────────────────────────
    // Check a few common product IDs
    const sampleProducts = [659232, 516998, 517044, 662183, 513721];

    console.log("");
    console.log("  ── Sample Markets ───────────────────────────");
    console.log("");
    console.log("  Product   Market PDA                                    Status");
    console.log("  ───────── ──────────────────────────────────────────── ──────");

    for (const pid of sampleProducts) {
        const market = marketPDA(programId, pid);
        const marketInfo = await connection.getAccountInfo(market);
        const marketStr = market.toString().slice(0, 44);
        const statusStr = marketInfo ? `initialized (${marketInfo.data.length} bytes)` : "NOT FOUND";
        console.log(`  ${String(pid).padEnd(9)} ${marketStr.padEnd(44)} ${statusStr}`);
    }

    console.log("");
}

main().catch((e) => {
    console.error("Error:", e);
    process.exit(1);
});
