import React, { FormEvent } from 'react';
import { alpha, Button, FormControl, FormGroup, TextField, useTheme } from '@mui/material';

export function ChangeService({ set_url }: { set_url: (arg0: string) => void; }) {
  const theme = useTheme();
  const service = React.useRef<HTMLInputElement | null>(null);
  const handle_click = (ev: FormEvent) => {
    ev.preventDefault();
    if (service.current !== null) {
      const url = new URL(global.location.toString());
      url.searchParams.set("service_url", service.current.value);
      global.location.href = url.toString();
      set_url(service.current.value);
    }
  };
  return <FormControl onSubmit={handle_click} size="small">
    <FormGroup row={true}>
      <TextField variant="filled" color="success" size="small" ref={service} name="service" type="text"
        style={{
          backgroundColor: alpha(theme.palette.common.white, 0.65),
        }}
      ></TextField>
      <Button variant="contained" color="success" type="submit">Set Server</Button>
    </FormGroup>
  </FormControl>;
}
