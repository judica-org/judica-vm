import FactoryIcon from '@mui/icons-material/Factory';
import { makeStyles, Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, createStyles } from '@mui/material';
import { Spacing } from '@mui/system';
import React from 'react';

export type NFTSale = {
  price: number,
  currency: any,
  seller: number,
  transfer_count: number,
}

// const useStyles = makeStyles(({ spacing }: {spacing: Spacing}) => createStyles({
//   card: {
//     marginTop: 40,
//     borderRadius: spacing(0.5),
//     transition: '0.3s',
//     width: '90%',
//     overflow: 'initial',
//     background: '#ffffff',
//   },
//   content: {
//     paddingTop: 0,
//     textAlign: 'left',
//     overflowX: 'auto',
//     '& table': {
//       marginBottom: 0,
//     }
//   },
//   root: ({ bgColor = 'primary.main', offset = '-40px', ...styles }) => ({
//     backgroundColor: 'grey',
//     borderRadius: spacing(2),
//     margin: `${offset} auto 0`,
//     width: '88%',
//     ...styles,
//   }),
//   title: {
//     color: 'white',
//     fontWeight: 'bold',
//   },
//   subheader: {
//     color: 'rgba(255, 255, 255, 0.76)',
//   },
// }));

// const EnergyExchange = React.memo(function EnergyExchangeWithHeader({ listings }: { listings: NFTSale[] }) {
  export const EnergyExchange = ({listings}: {listings: NFTSale[]}) => {

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