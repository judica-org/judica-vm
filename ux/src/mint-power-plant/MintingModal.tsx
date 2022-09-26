import { Close } from '@mui/icons-material';
import { Modal, Paper, IconButton, Divider, Button } from '@mui/material';
import { useEffect, useState } from 'react';
import MintingForm from './MintingForm';

function MintingModal({ location }: { location: [number, number]|null }) {
  const [open, setOpen] = useState(false);

  const handleClose = () => setOpen(false);
  const handleOpen = () => setOpen(true);

  return (
    <div className="MintingModal">
      <Button onClick={handleOpen}>Mint Power Plant</Button>
      <Modal
        aria-labelledby='minting-modal'
        aria-describedby='minting-power-plant-display'
        open={open}
        style={{
          position: 'absolute',
          width: 350
        }}
      >
        <Paper>
          <IconButton aria-label='close' style={{
            position: 'absolute',
            right: 1,
            top: 1,
            color: 'blue',
          }} onClick={handleClose}>
            <Close />
          </IconButton>
          <Divider />
         { location && <MintingForm location={location} />}
        </Paper>
      </Modal>
    </div>
  )
}

export default MintingModal;