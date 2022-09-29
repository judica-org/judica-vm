import { Card, CardHeader, CardContent, FormControl, TextField, Button, Typography } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import React from "react";
import { MaterialPriceDisplay, parse_trading_pair, trading_pair_to_string } from "../App";
import { SuccessfulTradeOutcome, tauri_host } from "../tauri_host";
import { RawMaterialsActions } from "../util";

const PurchaseMaterialForm = ({ action: action_in, market: market_in }: {
  readonly action: RawMaterialsActions;
  market: MaterialPriceDisplay;
}) => {
  const [action, set_action] = React.useState<RawMaterialsActions>(action_in);
  const [market, set_market] = React.useState<MaterialPriceDisplay>(market_in);
  const [trade_amt, set_trade_amt] = React.useState(0);
  const [formula_result, set_formula_result] = React.useState("");
  const formula = async (a: number) => {
    if (typeof market.price_a_b !== "number")
      return null;
    switch (action) {
      // TODO: Approximate via an invoke, which is much better
      case "SELL": {
        let outcome = await tauri_host.simulate_trade(market.trading_pair, [trade_amt, 0], "sell");
        let ok: SuccessfulTradeOutcome | undefined = outcome.Ok;
        if (ok) {
          return `Estimated to get ${ok.amount_player_purchased} ${ok.asset_player_purchased}`
        } else {
          return `${JSON.stringify(outcome.Err!)}`
        }
      }
      case "BUY": {

        let outcome = await tauri_host.simulate_trade(market.trading_pair, [trade_amt, 0], "buy");
        let ok: SuccessfulTradeOutcome | undefined = outcome.Ok;
        if (ok) {
          return `Estimated to cost ${ok.amount_player_sold} ${ok.asset_player_sold}`
        } else {
          return `${JSON.stringify(outcome.Err!)}`
        }

      }

    };
  };
  // Triggers a price check every period, or whenever a change in the forumla dependencies
  React.useEffect(() => {
    let cancel = false;
    let callback = (async () => {
      if (cancel) return;
      set_formula_result(await formula(trade_amt) ?? "");
      // update once a second...
      setTimeout(callback, 1000);
    });
    callback()
    return () => {
      cancel = true;
    };
  }, [trade_amt, market, action])

  const flip_market = () => {
    let pair = parse_trading_pair(market.trading_pair);
    let new_obj: MaterialPriceDisplay = {

      asset_a: market.asset_b,
      asset_b: market.asset_a,
      trading_pair: trading_pair_to_string(pair),
      price_a_b:
        typeof market.price_a_b === "number" ?
          1 / market.price_a_b :
          market.price_a_b,
      display_asset: market.display_asset,
    };
    set_market(new_obj);
  };

  const opposite_action = () => {

    switch (action) {
      // TODO: Approximate via an invoke, which is much better
      case "SELL":
        return ("BUY")
      case "BUY":
        return ("SELL")
    };
  }

  if (typeof market.price_a_b !== "number")
    return null;

  const handle_click = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {

    ev.preventDefault();
    if (trade_amt)
      tauri_host.make_move_inner({ trade: { amount_a: trade_amt, amount_b: 0, pair: market.trading_pair, sell: action === "SELL" } }, "0");
  };
  // for creater should be extracted out into a form util
  return <Card>
    <CardHeader
      title={action}
    >
    </CardHeader>
    <CardContent>
      <Typography variant="h6">
        Trading {market.asset_a} for {market.asset_b}
      </Typography>
      <div className='MoveForm' >
        <FormControl >
          <TextField label={<div>Estimate: {formula_result}</div>} type="number" value={trade_amt} onChange={(ev) => { set_trade_amt(parseInt(ev.target.value)) }}></TextField>
          <Button type="submit" onClick={handle_click}>{action}</Button>
          <Button onClick={() => flip_market()}>Flip Market</Button>
          <Button onClick={() => set_action(opposite_action())}>Switch To {opposite_action()} </Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};

export default PurchaseMaterialForm;

