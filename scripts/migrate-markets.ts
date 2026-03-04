/**
 * Migrate all existing markets from legacy (113 bytes) to extended (129 bytes) layout.
 * Sets max_price_deviation_bps for the M-05 price deviation circuit breaker.
 *
 * Must be run AFTER deploying the patched program (which adds the migrate_market instruction).
 *
 * Usage:
 *   bun --env-file=../.env run scripts/migrate-markets.ts
 */
import {
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    TransactionInstruction,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// ── Constants (must match program/src/constants.rs + lib.rs) ────────────────

const EXCHANGE_SEED = Buffer.from("exchange");
const MARKET_SEED = Buffer.from("market");
const DISC_MARKET = Buffer.from([0xdf, 0xb8, 0x9c, 0xee, 0x6b, 0xa0, 0x8a, 0x4a]);
const DISC_MIGRATE_MARKET = Buffer.from([0xc9, 0x71, 0xb5, 0x78, 0xd9, 0x3c, 0x6d, 0xcb]);

// 10% max deviation per oracle update — conservative starting point
const MAX_PRICE_DEVIATION_BPS = 1000n;

// ── Helpers ─────────────────────────────────────────────────────────────────

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

function marketPDA(programId: PublicKey, productId: number): PublicKey {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(productId, 0);
    return PublicKey.findProgramAddressSync([MARKET_SEED, buf], programId)[0];
}

function buildMigrateMarketIx(
    programId: PublicKey,
    market: PublicKey,
    exchangeState: PublicKey,
    admin: PublicKey,
    productId: number,
    maxDeviationBps: bigint,
): TransactionInstruction {
    // Data: disc(8) + product_id(4 LE) + max_price_deviation_bps(8 LE) = 20 bytes
    const data = Buffer.alloc(20);
    DISC_MIGRATE_MARKET.copy(data, 0);
    data.writeUInt32LE(productId, 8);
    data.writeBigUInt64LE(maxDeviationBps, 12);

    return new TransactionInstruction({
        programId,
        keys: [
            { pubkey: market, isSigner: false, isWritable: true },         // market: account_mut
            { pubkey: exchangeState, isSigner: false, isWritable: false }, // exchange_state: pda(EXCHANGE_SEED)
            { pubkey: admin, isSigner: true, isWritable: true },           // admin: signer_mut
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // system_program
        ],
        data,
    });
}

// ── Main ────────────────────────────────────────────────────────────────────

async function main() {
    const fileEnv = loadEnvFile(resolve(ROOT, ".env"));
    const env = { ...fileEnv, ...process.env };

    const programIdStr = env.PROGRAM_ID;
    const rpcUrl = env.SOLANA_RPC_URL;
    const keypairPath = env.KEYPAIR_PATH || resolve(ROOT, "keypair.json");

    if (!programIdStr || !rpcUrl) {
        console.error("ERROR: PROGRAM_ID and SOLANA_RPC_URL env vars are required");
        process.exit(1);
    }

    const programId = new PublicKey(programIdStr);
    const resolvedKeypairPath = keypairPath.startsWith("/")
        ? keypairPath
        : resolve(ROOT, keypairPath);

    if (!existsSync(resolvedKeypairPath)) {
        console.error(`ERROR: Keypair not found at ${resolvedKeypairPath}`);
        process.exit(1);
    }

    const raw = JSON.parse(readFileSync(resolvedKeypairPath, "utf-8"));
    const admin = Keypair.fromSecretKey(new Uint8Array(raw));
    const connection = new Connection(rpcUrl, "confirmed");
    const [exchangeState] = PublicKey.findProgramAddressSync([EXCHANGE_SEED], programId);

    console.log("");
    console.log("=== Migrate Markets to Extended Layout ===");
    console.log("");
    console.log("  Admin:          ", admin.publicKey.toString());
    console.log("  Program ID:     ", programId.toString());
    console.log("  Deviation BPS:  ", MAX_PRICE_DEVIATION_BPS.toString(), `(${Number(MAX_PRICE_DEVIATION_BPS) / 100}%)`);
    console.log("");

    // Discover all market accounts on-chain
    const accounts = await connection.getProgramAccounts(programId, {
        filters: [{ memcmp: { offset: 0, bytes: DISC_MARKET.toString("base64"), encoding: "base64" } }],
    });

    if (accounts.length === 0) {
        console.log("  No markets found on-chain.");
        return;
    }

    // Separate into legacy (needs migration) and already-migrated
    const legacy: { productId: number; pubkey: PublicKey }[] = [];
    const migrated: { productId: number; pubkey: PublicKey }[] = [];

    for (const { pubkey, account } of accounts) {
        const productId = account.data.readUInt32LE(8); // skip 8-byte discriminator
        if (account.data.length >= 129) {
            migrated.push({ productId, pubkey });
        } else {
            legacy.push({ productId, pubkey });
        }
    }

    console.log(`  Markets found:    ${accounts.length}`);
    console.log(`  Already migrated: ${migrated.length}`);
    console.log(`  Need migration:   ${legacy.length}`);
    console.log("");

    if (migrated.length > 0) {
        for (const m of migrated) {
            console.log(`  [SKIP] Product ${m.productId} — already 129 bytes`);
        }
    }

    if (legacy.length === 0) {
        console.log("  All markets already migrated. Nothing to do.");
        return;
    }

    // Build instructions for all legacy markets
    const instructions: TransactionInstruction[] = [];
    for (const m of legacy) {
        const expectedPDA = marketPDA(programId, m.productId);
        if (!expectedPDA.equals(m.pubkey)) {
            console.error(`  [ERROR] Product ${m.productId}: PDA mismatch. Expected ${expectedPDA}, got ${m.pubkey}`);
            continue;
        }
        instructions.push(
            buildMigrateMarketIx(programId, m.pubkey, exchangeState, admin.publicKey, m.productId, MAX_PRICE_DEVIATION_BPS)
        );
        console.log(`  [MIGRATE] Product ${m.productId} — ${m.pubkey.toString()}`);
    }

    if (instructions.length === 0) {
        console.log("  No valid migrations to perform.");
        return;
    }

    // Send as single transaction (9 small instructions fit easily)
    console.log("");
    console.log(`  Sending ${instructions.length} migrate_market instructions in one transaction...`);

    const { blockhash } = await connection.getLatestBlockhash("confirmed");
    const message = new TransactionMessage({
        payerKey: admin.publicKey,
        recentBlockhash: blockhash,
        instructions,
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

    console.log("  Migration complete.");
    console.log("    Signature:", sig);
    console.log("");
}

main().catch((e) => {
    console.error("Error:", e);
    process.exit(1);
});
