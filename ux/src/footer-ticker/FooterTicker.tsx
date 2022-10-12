// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Box, Typography } from '@mui/material';
import React from 'react'
import Ticker from 'react-ticker'

const FooterTicker = ({ player_status }: { player_status: string[] }) => {
  const status = player_status.join(" +++ ").concat("+++");

  return (
    <Box>
      <Ticker offset={0} height={40}>
        {() => (
          <>
            <Typography variant="h6" className="StatusTickerText">{status}</Typography>
          </>
        )}
      </Ticker>
    </Box>
  )
}
export default FooterTicker;