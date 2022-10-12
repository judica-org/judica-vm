// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
            <TableCell align="right">Cost in Virtual Sats</TableCell>
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