/**
 * Update max_leverage and max_open_interest for all on-chain markets.
 *
 * Defaults: max_leverage=20, max_open_interest=$10,000 (10_000_000_000 in USDC decimals).
 * Override via CLI args:
 *   bun --env-file=../.env run scripts/update-market-params.ts [max_leverage] [max_oi_usd]
 *
 * Examples:
 *   bun --env-file=../.env run scripts/update-market-params.ts          # 20x, $10k
 *   bun --env-file=../.env run scripts/update-market-params.ts 10 5000  # 10x, $5k
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

// ── Constants (must match program/src/constants.rs + lib.rs) ────────────────

const EXCHANGE_SEED = Buffer.from("exchange");
const MARKET_SEED = Buffer.from("market");
const DISC_MARKET = Buffer.from([0xdf, 0xb8, 0x9c, 0xee, 0x6b, 0xa0, 0x8a, 0x4a]);
const DISC_UPDATE_MARKET_PARAMS = Buffer.from([0x46, 0x75, 0xca, 0xbf, 0xcd, 0xae, 0x5c, 0x52]);

const USDC_DECIMALS = 6;

// ── Defaults ────────────────────────────────────────────────────────────────

const DEFAULT_MAX_LEVERAGE = 20;
const DEFAULT_MAX_OI_USD = 10_000; // $10k

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

function buildUpdateMarketParamsIx(
    programId: PublicKey,
    market: PublicKey,
    exchangeState: PublicKey,
    admin: PublicKey,
    productId: number,
    maxLeverage: number,
    maxOpenInterest: bigint,
): TransactionInstruction {
    // Data: disc(8) + product_id(4 LE) + max_leverage(4 LE) + max_open_interest(8 LE) = 24 bytes
    const data = Buffer.alloc(24);
    DISC_UPDATE_MARKET_PARAMS.copy(data, 0);
    data.writeUInt32LE(productId, 8);
    data.writeUInt32LE(maxLeverage, 12);
    data.writeBigUInt64LE(maxOpenInterest, 16);

    return new TransactionInstruction({
        programId,
        keys: [
            { pubkey: market, isSigner: false, isWritable: true },         // market: account_mut
            { pubkey: exchangeState, isSigner: false, isWritable: false }, // exchange_state: pda(EXCHANGE_SEED)
            { pubkey: admin, isSigner: true, isWritable: false },          // admin: signer
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

    // Parse CLI args
    const args = process.argv.slice(2);
    const maxLeverage = args[0] ? parseInt(args[0], 10) : DEFAULT_MAX_LEVERAGE;
    const maxOiUsd = args[1] ? parseInt(args[1], 10) : DEFAULT_MAX_OI_USD;
    const maxOpenInterest = BigInt(maxOiUsd) * BigInt(10 ** USDC_DECIMALS);

    if (maxLeverage < 1 || maxLeverage > 50) {
        console.error("ERROR: max_leverage must be between 1 and 50");
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
    console.log("=== Update Market Parameters ===");
    console.log("");
    console.log("  Admin:             ", admin.publicKey.toString());
    console.log("  Program ID:        ", programId.toString());
    console.log("  Max Leverage:      ", `${maxLeverage}x`);
    console.log("  Max Open Interest: ", `$${maxOiUsd.toLocaleString()} (${maxOpenInterest.toString()} raw)`);
    console.log("");

    // Discover all market accounts on-chain
    const accounts = await connection.getProgramAccounts(programId, {
        filters: [{ memcmp: { offset: 0, bytes: DISC_MARKET.toString("base64"), encoding: "base64" } }],
    });

    if (accounts.length === 0) {
        console.log("  No markets found on-chain.");
        return;
    }

    console.log(`  Markets found: ${accounts.length}`);
    console.log("");

    // Build instructions
    const instructions: TransactionInstruction[] = [];
    for (const { pubkey, account } of accounts) {
        const d = account.data.slice(8); // skip discriminator
        const productId = d.readUInt32LE(0);
        const currentLev = d.readUInt32LE(36);
        const currentOI = Number(d.readBigUInt64LE(40)) / 10 ** USDC_DECIMALS;

        console.log(`  Product ${productId}: ${currentLev}x / $${currentOI.toLocaleString()} -> ${maxLeverage}x / $${maxOiUsd.toLocaleString()}`);

        instructions.push(
            buildUpdateMarketParamsIx(programId, pubkey, exchangeState, admin.publicKey, productId, maxLeverage, maxOpenInterest)
        );
    }

    // Send as single transaction
    console.log("");
    console.log(`  Sending ${instructions.length} update_market_params instructions in one transaction...`);

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

    console.log("  Parameters updated.");
    console.log("    Signature:", sig);
    console.log("");
}

main().catch((e) => {
    console.error("Error:", e);
    process.exit(1);
});
