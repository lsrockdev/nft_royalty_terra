import {
  LCDClient,
  MnemonicKey,
  MsgExecuteContract,
	Coins,
	Coin
} from "@terra-money/terra.js";
import info from "./constant";
import fetch from 'isomorphic-fetch';

(async () => {
	try {
		// Create LCDClient for Bombay-12 TestNet
		const gasPrices = await (await fetch('https://bombay-fcd.terra.dev/v1/txs/gas_prices')).json();
		const gasPricesCoins = new Coins(gasPrices);
		const terra: LCDClient = new LCDClient({
			URL: info.NETWORK,
			chainID: info.CHAIN_ID,
			gasPrices: gasPricesCoins,
			gasAdjustment: "1.5",
		});

		// Get deployer wallet
		const wallet = terra.wallet(new MnemonicKey({ mnemonic: info.WALLET_SEEDS }));
		console.log("Wallet: ", wallet.key.accAddress);

		let timeStamp = Math.floor(Date.now() / 1000 ) + 3600 * 24 * 7
		console.log('timeStamp', timeStamp)
		const expire_at = {
				// "at_time": "1668544526734254325", // 19 digits format
				"never": {}
		},
		price = {
			"amount": "200", //0.000001 Luna
			"info": {
				"native_token": {"denom": "uluna"}
			}
		}

		const setBuyCriteriaMsg = { set_buycriteria:
			{
				auction_rate: "0.1",
				is_auction: false,
				is_loot_box: true,
				max_price: "10000",
				nft_address: "terra1rmw87h769rt553myzcvnqavvnqzqxm2r9twsju",
				protection_period: 10,
				protection_rate: "0.1",
				token_id: "1"
			}
		}
		const setSellCriteriaMsg = { set_sellcriteria: 
			{ 
				above_price_rate: "0.1",
				amount_mft: "200",
				is_auction: false,
				is_loot_box: true,
				offer_time: timeStamp,
				protection_rate: "0.1",
				selling_time: timeStamp
			}
		}
		const buyNFTCriteriaMsg = {
			buy_nft_on_criteria: {
				order_id: 4
			}
		}
		console.log('wallet address', wallet.key.accAddress)
		const increase = new MsgExecuteContract(
			wallet.key.accAddress, // sender
			"terra10v7hlw7cz7rvhht5vkgg9vkcvull3snnhrql50",
			buyNFTCriteriaMsg,
			// [
			// 	new Coin("uusd", "550")
			// ]
		)

		const increaseTx = await wallet.createAndSignTx({
			msgs: [increase]
		})
		console.log("increaseTx?", increaseTx && increaseTx?.body.messages)
		if (increaseTx) {
			const increaseTxTxResult = await terra.tx.broadcast(increaseTx);
			console.log("increaseTxTxResult?", increaseTxTxResult)
		}
	} catch (e) {
		console.log(e)
	}
})();