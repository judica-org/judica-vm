import React, { useState, useEffect } from "react";
import { listen } from '@tauri-apps/api/event';
import { Button, FormControl, FormControlLabel, FormLabel, Grid, Radio, RadioGroup, Switch, TextField } from "@mui/material";

const MintingForm = () => {
  const [location, setLocation] = useState([]);
  const [superMint, setSuperMint] = useState(false);

  const defaultValues = {
    plant_type: 'Select',
    scale: 1,
    location,
  }

  useEffect(() => {
    listen("globe-click", (ev: { payload: any }) => {
      console.log(["globe-click"], ev);
      setLocation(ev.payload);
    });
  });


  /* Will need two submit buttons:
    1. simulate - will submit formData with simulate = true. 
    2. mint - will submit formData with simulate = false.
    3. toggle for mint vs. super-mint
       estimate display should just return a list that we can populate dynamically. 
  */

  const [formValues, setFormValues] = useState(defaultValues);
  // fix this type
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setFormValues({
      ...formValues,
      [name]: value,
    });
  };

  const handleSelectChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSuperMint(event.target.checked);
  };

  const handleSubmit = (event: React.SyntheticEvent) => {
    event.preventDefault();
    console.log({...formValues, superMint});
  };

  return (
    <form onSubmit={handleSubmit}>
      <Grid container style={{ alignItems: "center" }}>
        <Grid item>
          <TextField
            id="scale-input"
            name="scale"
            label="Scale"
            type="number"
            value={formValues.scale}
            onChange={handleInputChange}
          />
        </Grid>
        <Grid item>
          <FormControl>
            <FormLabel>Plant Type</FormLabel>
            <RadioGroup
              name="plant-type"
              value={formValues.plant_type}
              onChange={handleInputChange}
              row
            >
              <FormControlLabel
                key="solar"
                value="Solar"
                control={<Radio size="small" />}
                label="Solar"
              />
              <FormControlLabel
                key="hydro"
                value="Hydro"
                control={<Radio size="small" />}
                label="Hydro"
              />
              <FormControlLabel
                key="flare"
                value="Flare"
                control={<Radio size="small" />}
                label="Flare"
              />
            </RadioGroup>
          </FormControl>
        </Grid>
        <Grid item>
          <div style={{ width: "400px" }}>
            Super Mint?
            <Switch
              checked={superMint}
              onChange={handleSelectChange}
              inputProps={{ 'aria-label': 'controlled' }}
            />
          </div>
        </Grid>
        <Button variant="contained" color="primary" type="submit">
          Estimate
        </Button>
        <Button variant="contained" color="primary" type="submit">
          Mint
        </Button>
      </Grid>
    </form>
  );
};
export default MintingForm;