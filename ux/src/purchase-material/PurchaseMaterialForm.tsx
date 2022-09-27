import { Card, CardHeader, CardContent, FormControl, TextField, Button } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import { invoke } from "@tauri-apps/api";
import React from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { MaterialPriceDisplay } from "../App";
import { tauri_host } from "../tauri_host";
import { TradingPairID } from "../Types/GameMove";
import { RawMaterialsActions } from "../util";

const PurchaseMaterialForm = ({ action, market }: {
  readonly action: RawMaterialsActions;
  readonly market: MaterialPriceDisplay;
}) => {
  const [trade_amt, set_trade_amt] = React.useState(0);
  const handle_submit = (ev: React.FormEvent<HTMLDivElement>): void => {
    // TODO: Detect Trade Direction
    ev.preventDefault();
    if (trade_amt)
      tauri_host.make_move_inner({ trade: { amount_a: trade_amt, amount_b: 0, pair: market.trading_pair, sell: action === "SELL" } }, 0)
  };

  const formula = (a: number) => {
    if (typeof market.price !== "number")
      return null;
    switch (action) {
      case "SELL":
        return `Recieving ${a / market.price} Units`;
      case "BUY":
        return `Buying ${a * market.price} Units`;

    };
  };
  if (typeof market.price !== "number")
    return null;

  // for creater should be extracted out into a form util
  return <Card>
    <CardHeader
      title={action}
    >
    </CardHeader>
    <CardContent>
      <div className='MoveForm' >
        <FormControl onSubmit={handle_submit}>
          <TextField label={<div>Estimate: {formula(trade_amt)}</div>} type="number" value={trade_amt} onChange={(ev) => { set_trade_amt(parseInt(ev.target.value)) }}></TextField>
          <Button type="submit">{action}</Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};

export default PurchaseMaterialForm;

