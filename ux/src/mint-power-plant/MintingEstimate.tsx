import { Typography, Table, TableHead, TableRow, TableCell, TableBody } from "@mui/material"

export const MintingEstimate = ({ costs }: { costs: any[] }) => {
  return (
    <div className="Estimate">
      <Typography variant='h6'>Materials Cost Estimate</Typography>
      <Table>
        <TableHead>
          <TableRow>
            <TableCell>Material</TableCell>
            <TableCell align="right">Quantity Needed</TableCell>
            <TableCell align="right">Cost in BTC</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {costs.map((cost, index) => (
            <TableRow key={index}>
              <TableCell component="th" scope="row">
                {cost[0]}
              </TableCell>
              <TableCell align="right">{cost[1]}</TableCell>
              <TableCell align="right">{cost[2]}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}