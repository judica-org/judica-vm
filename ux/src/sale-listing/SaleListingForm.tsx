// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Card, CardHeader, CardContent, FormControl, TextField, Typography, Button } from "@mui/material";
import { useState } from "react";
import { tauri_host } from "../tauri_host";

const SaleListingForm = ({ nft_id, currency }: { nft_id: string, currency: string | null }) => {
  const [sale_price, set_sale_price] = useState<number>(0);

  const handle_submit = () => {
    if (sale_price > 0 && currency) {
      tauri_host.make_move_inner({ list_n_f_t_for_sale: { nft_id, currency, price: sale_price } });
    }
  };

  return <Card>
    <CardHeader
      title={'Sell?'}
      subheader={`List Plant ${nft_id} For Sale`}
    >
    </CardHeader>
    <CardContent>
      <div className='MoveForm' >
        <FormControl>
          <Typography variant="body2">Sale Price in Virtual BTC</Typography>
          <TextField type="number" value={sale_price} onChange={(ev) => { set_sale_price(parseInt(ev.target.value)) }}></TextField>
          <Button variant="contained" type="submit" onClick={handle_submit}>Create Listing</Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};

export default SaleListingForm;