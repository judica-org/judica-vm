import { Card, CardHeader, CardContent, Typography, Button, TextField, FormControl } from "@mui/material";
import { useEffect, useState } from "react";
import { tauri_host } from "../tauri_host";

const PurchaseOfferForm = ({ nft_id, currency, listing_price }: { nft_id: string, currency: string | null, listing_price: number | null }) => {
  const [limit_price, set_limit_price] = useState<number>(0);
  useEffect(() => {
    set_limit_price(listing_price ?? 0);
  }, [listing_price])

  const handle_submit = () => {
    if (limit_price && currency)
      tauri_host.make_move_inner({ purchase_n_f_t: { currency, limit_price, nft_id } });
    console.log(['purchase-nft'], { purchase_n_f_t: { currency, limit_price, nft_id } });
  };

  return <Card>
    <CardHeader
      title={'Purchase?'}
      subheader={`Make an offer to purchase Plant ${nft_id}`}
    >
    </CardHeader>
    <CardContent>
      <div className='MoveForm' >
        <FormControl>
          <Typography variant="body2">Limit Price</Typography>
          <TextField type="number" value={limit_price} onChange={(ev) => { set_limit_price(parseInt(ev.target.value)) }}></TextField>
          <Button variant="contained" type="submit" onClick={handle_submit}>Make Purchase Offer</Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};

export default PurchaseOfferForm;