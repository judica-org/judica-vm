import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody } from '@mui/material';
import FormModal from '../form-modal';

export type NFTSale = {
  price: number,
  currency: any,
  seller: number,
  transfer_count: number,
  // add plant type
}

// const EnergyExchange = React.memo(function EnergyExchangeWithHeader({ listings }: { listings: NFTSale[] }) {
export const EnergyExchange = ({ listings }: { listings: NFTSale[] }) => {

  // const classes = useStyles();
  return (
    <div>
      <div className='energy-exchange-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            // classes={cardHeaderStyles}
            title={'Energy Exchange'}
            subheader={'Power Plants For Sale'}
          />
          <CardContent className={'content'}>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Plant Type</TableCell>
                  <TableCell>Seller</TableCell>
                  <TableCell align="right">Price ($)</TableCell>
                  <TableCell align="right">Currency (token)</TableCell>
                  <TableCell align="right">Transfer Count</TableCell>
                  <TableCell align="right"></TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {listings.map((listing, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      {/* color code these in the future */}
                      <FactoryIcon className='sale-factory-icon' />
                    </TableCell>
                    <TableCell component="th" scope="row">
                      {listing.seller}
                    </TableCell>
                    <TableCell align="right">{listing.price}</TableCell>
                    <TableCell align="right">{listing.currency}</TableCell>
                    <TableCell align="right">{listing.transfer_count}</TableCell>
                    <TableCell align="right"><FormModal title={"Purchase"} /></TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
    </div>
  )
  // });
};
export default EnergyExchange;