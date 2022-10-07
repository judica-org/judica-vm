import ConstructionIcon from '@mui/icons-material/Construction';
import { Typography, Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody } from '@mui/material';
import { MaterialPriceDisplay } from '../App';
import FormModal from '../form-modal/FormModal';
import { material_type_color_map } from '../util';


export const RawMaterialsMarket = ({ materials }: { materials: MaterialPriceDisplay[] }) => {
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
                  <TableCell align="right">Price Per Unit</TableCell>
                  <TableCell align="right">Currency (token)</TableCell>
                  <TableCell align="right">Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {materials && materials.map((material, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <ConstructionIcon className='sale-factory-icon' fontSize={'medium'} />
                      <Typography>{material.display_asset}</Typography>
                    </TableCell>
                    <TableCell align="right">1 to {material.price_a_b_b_a[0]}</TableCell>
                    <TableCell align="right">{material.asset_a} / {material.asset_b}</TableCell>
                    <TableCell align="right">
                      <FormModal title={"Buy Tokens"} action={"BUY"} market={material} />
                      <FormModal title={"Sell Tokens"} action={"SELL"} market={material} />
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