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