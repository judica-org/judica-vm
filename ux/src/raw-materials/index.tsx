import ConstructionIcon from '@mui/icons-material/Construction';
import { Typography, Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect } from 'react';
import FormModal from '../form-modal';
import { material_type_color_map } from '../util';

export type MaterialPriceData = {
  readonly trading_pair: {
    readonly asset_a: number;
    readonly asset_b: number;
  },
  readonly asset_a: string;
  readonly mkt_qty_a: number;
  readonly asset_b: string;
  readonly mkt_qty_b: number;
}

type MaterialType = 'Steel' | 'Silicon' | 'Concrete';

type MaterialPriceDisplay = {
  trading_pair: {
    asset_a: number;
    asset_b: number;
  },
  material_type: MaterialType;
  price: number | 'not available';
  currency: string;
}

export const RawMaterialsMarket = () => {

  const [materials, set_materials] = useState<MaterialPriceDisplay[] | null>(null);
  useEffect(() => {
    const unlisten = appWindow.listen("materials-price-data", (ev) => {
      console.log(ev);
      const materials_data = ev.payload as MaterialPriceData[];
      const transformed: MaterialPriceDisplay[] = materials_data.map(({ trading_pair, asset_a, mkt_qty_a, asset_b, mkt_qty_b }) => {
        return {
          trading_pair,
          material_type: asset_a as MaterialType,
          price: Math.round(mkt_qty_b / mkt_qty_a),
          currency: asset_b,
        }
      })
      set_materials(transformed)
    });

    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  }, [materials]);



  return (
    <div>
      <div className='materials-market-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            title={'Raw Meterials Market'}
            subheader={'Buy and sell resources to build power plants'}
          />
          <CardContent className={'content'}>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Material</TableCell>
                  <TableCell align="right">Price Per Kg</TableCell>
                  <TableCell align="right">Currency (token)</TableCell>
                  <TableCell align="right">Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {materials && materials.map((material, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <ConstructionIcon className='sale-factory-icon' sx={{ color: material_type_color_map[material.material_type] }} fontSize={'medium'} />
                      <Typography>{material.material_type}</Typography>
                    </TableCell>
                    <TableCell align="right">{material.price}</TableCell>
                    <TableCell align="right">{material.currency}</TableCell>
                    <TableCell align="right">
                      <FormModal title={"Purchase Materials"} currency={material.currency} material_type={material.material_type} />
                      <FormModal title={"Sell Materials"} currency={material.currency} material_type={material.material_type} />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
    </div>
  )
};
export default RawMaterialsMarket;