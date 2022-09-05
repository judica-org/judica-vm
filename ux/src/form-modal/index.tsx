import { useState } from 'react';
import { Modal, Button, Paper, IconButton } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import PurchaseMaterialForm from '../purchase-material';
import { RawMaterialsActions } from '../util';

function FormModal({ title, currency, material_type }: { readonly title: RawMaterialsActions; readonly currency?: number; material_type: string }) {
  const [open, setOpen] = useState(false);

  const CustomModal = () => {
    return (
      <Modal
        aria-labelledby='simple-modal-title'
        aria-describedby='simple-modal-description'
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
            <CloseIcon />
          </IconButton>
          {title === 'Purchase' ? <PurchaseMaterialForm action={title} subtitle={`Purchase ${material_type} from the market?`} currency={currency} /> : <PurchaseMaterialForm action={title} subtitle={`Sell some of your ${material_type}?`} currency={currency} />}
        </Paper>
      </Modal>
    )
  };

  const handleOpen = () => {
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  return (
    <div>
      <div>
        <Button
          onClick={() => {
            handleOpen();
          }}
        >
          {title}
        </Button>
      </div>
      <CustomModal />
    </div>
  );
}

export default FormModal;