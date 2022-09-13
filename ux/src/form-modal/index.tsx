import { useState } from 'react';
import { Modal, Button, Paper, IconButton } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import PurchaseMaterialForm from '../purchase-material';
import PurchaseOfferForm from '../purchase-offer';
import { RawMaterialsActions } from '../util';
import SaleListingForm from '../sale-listing';

type FormModalProps = {
  readonly title: RawMaterialsActions | 'Purchase Plant' | 'Sell Plant';
  readonly currency: string;
  readonly material_type?: string;
  readonly nft_id?: number
};

function FormModal({ title, currency, material_type, nft_id }: FormModalProps) {
  const [open, setOpen] = useState(false);

  const pickForm = (title: string) => {
    switch (title) {
      case 'Purchase Materials':
        return <PurchaseMaterialForm action={title} subtitle={`Purchase ${material_type as string} from the market?`} currency={currency} />;
      case 'Sell Materials':
        return <PurchaseMaterialForm action={title} subtitle={`Sell some of your ${material_type as string}?`} currency={currency} />;
      case 'Purchase Plant':
        return <PurchaseOfferForm subtitle={`Purchase plant ${nft_id as number}`} />;
      case 'Sell Plant':
        return <SaleListingForm subtitle={'Sell'} />
    }
  }

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
          {
            pickForm(title)}
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