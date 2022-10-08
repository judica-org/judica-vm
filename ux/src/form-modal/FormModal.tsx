import { useState } from 'react';
import { Modal, Button, Paper, IconButton } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import PurchaseMaterialForm from '../purchase-material/PurchaseMaterialForm';
import { RawMaterialsActions } from '../util';
import { EntityID, TradingPairID } from '../Types/GameMove';
import { MaterialPriceDisplay } from '../App';

type NFTActions = 'Purchase Plant' | 'Sell Plant';
type FormModalProps = ({
  readonly action: RawMaterialsActions;
  readonly market: MaterialPriceDisplay;
} | {
  readonly action: NFTActions;
  readonly nft_id: EntityID;
  readonly currency: EntityID | null;
}) & { readonly title?: string };

function FormModal(props: FormModalProps) {
  const [open, setOpen] = useState<boolean>(false);

  const pickForm = (props: FormModalProps) => {
    switch (props.action) {
      case 'BUY':
        return <PurchaseMaterialForm action={props.action} market={props.market}></PurchaseMaterialForm>
      case 'SELL':
        return <PurchaseMaterialForm action={props.action} market={props.market}></PurchaseMaterialForm>
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
            pickForm(props)}
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
          {props.title ?? props.action}
        </Button>
      </div>
      <CustomModal />
    </div>
  );
}

export default FormModal;