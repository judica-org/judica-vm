import { Card, CardHeader, CardContent, FormControl, TextField, Typography, Button } from "@mui/material";
import { useState } from "react";
import { tauri_host } from "../tauri_host";

const SaleListingForm = ({ nft_id, currency }: { nft_id: string, currency: string | null }) => {
  const [sale_price, set_sale_price] = useState<number>(0);

  useEffect(() => {
    (async () => {
      tauri_host.get_material_schema
      setSchema(await invoke("get_listing_schema"));
    })()
  }, []);
  console.log("listing schema:", schema);

  const handle_submit = (data: FormSubmit) => {
    tauri_host.make_move_inner({ list_n_f_t_for_sale: data.formData })
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