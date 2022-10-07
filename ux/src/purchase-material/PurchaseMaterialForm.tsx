import { Card, CardHeader, CardContent, FormControl, TextField, Button, Typography, ToggleButtonGroup, ToggleButton } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import React from "react";
import { MaterialPriceDisplay } from "../App";
import { SuccessfulTradeOutcome, tauri_host, UnsuccessfulTradeOutcome } from "../tauri_host";
import { RawMaterialsActions } from "../util";

const PurchaseMaterialForm = ({ action: action_in, market }: {
  readonly action: RawMaterialsActions;
  market: MaterialPriceDisplay;
}) => {
  const [action, set_action] = React.useState<RawMaterialsActions>(action_in);
  const [market_flipped, set_market_flipped] = React.useState<boolean>(false);
  const [trade_amt, set_trade_amt] = React.useState<number>(0);
  const [limit_pct, set_limit_pct] = React.useState<number|null>(null);
  const [formula_result, set_formula_result] = React.useState("");
  const handle_error = (e: UnsuccessfulTradeOutcome) => {
    switch (typeof e) {
      case 'string':
        return "Market Slipped"
      case "object":
        if (e.InvalidTrade) return `Invalid Trade: ${e.InvalidTrade}`
        if (e.InsufficientTokens) return `Insufficient Tokens: ${e.InsufficientTokens}`
    }
  }
  const formula = async (a: number) => {
    if (isNaN(a)) return "Invalid Trade Entered";
    if (trade_amt === 0) return "No Trade Entered";
    let trade: [number, number] = [trade_amt, 0];
    if (market_flipped) trade.reverse();
    switch (action) {
      // TODO: Approximate via an invoke, which is much better
      case "SELL": {
        let outcome = await tauri_host.simulate_trade(market.trading_pair, trade, "sell");
        let ok: SuccessfulTradeOutcome | undefined = outcome.Ok;
        if (ok) {
          return `Estimated to sell ${ok.amount_player_sold} ${ok.asset_player_sold} to purchase ${ok.amount_player_purchased} ${ok.asset_player_purchased}`
        } else return handle_error(outcome.Err!);
      }
      case "BUY": {

        let outcome = await tauri_host.simulate_trade(market.trading_pair, trade, "buy");
        let ok: SuccessfulTradeOutcome | undefined = outcome.Ok;
        if (ok) {
          return `Estimated to purchase ${ok.amount_player_purchased} ${ok.asset_player_purchased} by selling ${ok.amount_player_sold} ${ok.asset_player_sold}`
        } else return handle_error(outcome.Err!);
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
  }, [trade_amt, market_flipped, action])

  const flip_market = () => {
    set_market_flipped(!market_flipped);
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


  const handle_click = async (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): Promise<void> => {

    ev.preventDefault();
    if (trade_amt && !isNaN(trade_amt)) {
      const trade: [number, number] = [trade_amt, 0];
      if (market_flipped) trade.reverse();
      let outcome = await tauri_host.simulate_trade(market.trading_pair, trade, action === "SELL" ? "sell" : "buy");
      let ok = outcome.Ok;
      if (ok) {
        // TODO: Add a flexible Cap for Limit Orders, fixed to +/- 10%.
        let cap = limit_pct === null ? null :
          Math.round(action === "SELL" ? (ok.amount_player_purchased * (1 - limit_pct)) : (ok.amount_player_sold * (1 + limit_pct)));
        if (confirm((action === "SELL" ?
          `Sell Will trade ${ok.amount_player_sold} ${ok.asset_player_sold} for at least ${cap} ${ok.asset_player_purchased}` :
          `Buy Will get ${ok.amount_player_purchased} ${ok.asset_player_purchased} for at most ${cap} ${ok.asset_player_sold}`)
          + `\n Slip tolerance ${limit_pct ?? 0 * 100}% from expected`
        ))
          tauri_host.make_move_inner({ trade: { amount_a: trade[0], amount_b: trade[1], pair: market.trading_pair, sell: action === "SELL", cap } });
        console.log(["trade-submitted"], { trade: { amount_a: trade[0], amount_b: trade[1], pair: market.trading_pair, sell: action === "SELL", cap } })
      } else {
        alert("Trade will not succeed, " + JSON.stringify(outcome.Err!))
      }
    }
  };
  const parse_limit_pct = (ev: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>): void => {
    let f = parseFloat(ev.target.value);
    set_limit_pct(isNaN(f) ? 0 : f);
  };
  // for creater should be extracted out into a form util
  return <Card>
    <CardHeader
      title={action}
    >
    </CardHeader>
    <CardContent>
      <Typography variant="h6">
        {action} {market_flipped ? market.asset_b : market.asset_a} for {market_flipped ? market.asset_a : market.asset_b}
      </Typography>
      <div className='MoveForm' >
        <FormControl >
          <ToggleButtonGroup
            color="warning"
            value={action}
            exclusive
            onChange={(ev, v) => v && set_action(v)}
            aria-label="buy or sell"
          >
            <ToggleButton value="BUY" aria-label="buy">
              Buy
            </ToggleButton>
            <ToggleButton value="SELL" aria-label="sell">
              Sell
            </ToggleButton>
          </ToggleButtonGroup>

          <ToggleButtonGroup
            color="success"
            value={`${market_flipped}`}
            exclusive
            onChange={(ev, v) => v && set_market_flipped(v === "true")}
            aria-label="main asset"
          >
            <ToggleButton value="false" aria-label={market.asset_a}>
              {market.asset_a}
            </ToggleButton>
            <ToggleButton value="true" aria-label={market.asset_b}>
              {market.asset_b}
            </ToggleButton>
          </ToggleButtonGroup>
          <TextField type="number" value={trade_amt} onChange={(ev) => { set_trade_amt(parseInt(ev.target.value)) }}></TextField>
          <Typography>
            {formula_result}
          </Typography>
          <TextField label={"Slip Tolerance (e.g. 0.1 => 10%)"} type="number"
            value={limit_pct} onChange={parse_limit_pct}></TextField>
          <Button type="submit" onClick={handle_click}>Execute {action}</Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};

export default PurchaseMaterialForm;

