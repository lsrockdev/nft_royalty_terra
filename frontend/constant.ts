
require('dotenv').config()
const NETWORK = "https://bombay-lcd.terra.dev";
const CHAIN_ID = "bombay-12";
const WALLET_SEEDS = process.env.WALLET_SEEDS
export default {
  NETWORK,
  CHAIN_ID,
  WALLET_SEEDS,
};
