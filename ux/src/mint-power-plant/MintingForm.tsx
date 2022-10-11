import React, { useState } from "react";
import { Button, Divider, FormControl, FormControlLabel, FormLabel, Grid, Radio, RadioGroup, Switch, TextField, Typography } from "@mui/material";
import { MintingEstimate } from "./MintingEstimate";
import { tauri_host, UnsuccessfulTradeOutcome } from "../tauri_host";
import { PlantType } from "../App";
import { handle_error } from "../purchase-material/PurchaseMaterialForm";

export const COORDINATE_PRECISION = 1000000;
const standardizeCoordinates = ({ lat, lng }: { lat: number, lng: number }): [number, number] => {
  // fix to 6 decimal places to conform with hex data then remove decimals
  const newLat = parseFloat(lat.toFixed(6)) * COORDINATE_PRECISION;
  const newLng = parseFloat(lng.toFixed(6)) * COORDINATE_PRECISION;

  return [newLat, newLng]
}

const MintingForm = ({ location }: { location: [number, number] }) => {
  const [superMint, setSuperMint] = useState(false);
  const [estimate, setEstimate] = useState<any[] | null>(null);

  const defaultValues = {
    plant_type: 'Select',
    scale: 1,
    location,
  }

  const [formValues, setFormValues] = useState(defaultValues);
  // fix this type
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value, valueAsNumber } = e.target;
    setFormValues({
      ...formValues,
      [name]: name === 'scale' ? valueAsNumber : value,
    });
  };

  const handleSelectChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSuperMint(event.target.checked);
  };

  const handleSubmit = async (event: any) => {
    event.preventDefault();
    const submitter_id = event.nativeEvent.submitter.id;
    const { scale, plant_type } = formValues;
    console.log(["submit-clicked"], { ...formValues, superMint }, { submitter_id });
    console.log(["number-log"], standardizeCoordinates({ lat: location[0], lng: location[1] }));
    if (submitter_id === "estimate") {
      try {
        let costs = await tauri_host.mint_power_plant_cost(scale, standardizeCoordinates({ lat: location[0], lng: location[1] }), plant_type as PlantType);
        console.log(["mint-plant-estimate"], costs)
        setEstimate(costs as unknown as any);
      } catch (e: any) {
        console.warn(e);
        alert(handle_error(e.TradeError as UnsuccessfulTradeOutcome));
      }
    }
    if (submitter_id === "mint") {
      // this expects entityID that isn't used. Remove later.
      if (plant_type === "Solar" || plant_type === "Hydro" || plant_type === "Flare")
        await tauri_host.super_mint(scale, standardizeCoordinates({ lat: location[0], lng: location[1] }), plant_type!);
    }
  };

  return (
    <div className="MintingForm">
      {estimate && <MintingEstimate costs={estimate}></MintingEstimate>}
      <Divider />
      <div>
        <Typography variant="h6"> Estimate Plant Cost and Build </Typography>
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
                  name="plant_type"
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
                Super Build?
                <Switch
                  checked={superMint}
                  onChange={handleSelectChange}
                  inputProps={{ 'aria-label': 'controlled' }}
                />
              </div>
            </Grid>
            <Button variant="contained" color="primary" type="submit" id="estimate">
              Estimate
            </Button>
            <Button variant="contained" color="primary" type="submit" id="mint">
              Build
            </Button>
          </Grid>
        </form>
      </div>
    </div >
  );
};
export default MintingForm;