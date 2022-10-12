// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { appWindow } from '@tauri-apps/api/window';
import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent, Icon, Divider, Typography } from '@mui/material';
import { emit } from '@tauri-apps/api/event';
import { fireSvg, solarSvg, hydroSvg } from './util';
import { PlantSelected, PlantType } from './App';
import { PlantOwnerSelect, PlantTypeSelect } from './GlobeHelpers';
import { COORDINATE_PRECISION } from './mint-power-plant/MintingForm';
import { EntityID } from './Types/GameMove';
import { UXPlantData } from './Types/Gameboard';
const { useState, useEffect } = React;

const scaling_formula = (units: number, max_units: number): number => {
    return units / max_units
}
type Plant = UXPlantData;
const stub_plant_data: Plant[] = [{
    coordinates: [46818188, 8227512],
    for_sale: false,
    hashrate: 90000,
    id: "45637",
    miners: 32,
    owner: "12345566",
    plant_type: "Flare",
    watts: 345634,
}, {
    coordinates: [49817492, 15472962],
    for_sale: false,
    hashrate: 32700,
    id: "13436",
    miners: 206,
    owner: "9494384",
    plant_type: "Hydro",
    watts: 67897,
}, {
    coordinates: [9145125, 40489673],
    for_sale: true,
    hashrate: 14100,
    id: "95944",
    miners: 30,
    owner: "12345566",
    plant_type: "Solar",
    watts: 23795,
}];

const memo_colors: Record<string, string> = {};
function memoized_color(name: string) {
    const color = memo_colors[name];
    if (color)
        return color;
    else
        memo_colors[name] = `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`;
    return memo_colors[name];
}

type BarData = { coordinates: number[], hashboards?: number, scale?: number, id: string };

function getBarData(plants: (UXPlantData)[]) {
    return plants.reduce<BarData[]>((acc, { coordinates, miners, watts, id }) => {
        return [...acc, { id, coordinates, scale: watts / 100_000 }, { id, coordinates: [coordinates[0] + 200000, coordinates[1] + 200000], hashboards: miners }];
    }, []);
}

const chose_color = (d: BarData): string => {
    if (d.hashboards && d.hashboards > 0) {
        return 'red'
    } else {
        return 'lightgreen'
    }
}

export default (props: { power_plants: UXPlantData[], user_id: EntityID | null }) => {
    const [all_plant_owners, set_all_plant_owners] = useState<Record<EntityID, true>>(
        {}
    ); // default to all owners
    const [selectedPlantOwners, setSelectedPlantOwners] = useState<Record<EntityID, boolean>>(
        {}
    ); // default to all owners
    const [location, setLocation] = useState<{ lat: number, lng: number } | null>(null);
    const [plantTypes, setPlantTypes] = React.useState<Record<PlantType, boolean>>({
        'Hydro': true,
        'Solar': true,
        'Flare': true
    });

    const [selected_plants, set_selected_plants] = React.useState<UXPlantData[]>(props.power_plants);
    const [output_bars, set_output_bars] = React.useState<BarData[]>(getBarData(props.power_plants));
    const [max_scale, set_max_scale] = React.useState(1);

    React.useEffect(() => {
        const plants_by_type = props.power_plants.filter(({ plant_type, owner }) => plantTypes[plant_type] && selectedPlantOwners[owner]);
        const output_bar_data = getBarData(plants_by_type);
        set_selected_plants(plants_by_type);
        set_output_bars(output_bar_data);
    }, [plantTypes, selectedPlantOwners]);

    React.useEffect(() => {
        const new_all: Record<EntityID, true> =
            Object.fromEntries(props.power_plants.map((plant) => [plant.owner, true]));
        set_all_plant_owners(
            new_all
        );
        const wattages = props.power_plants.map((p) => p.watts / 100000);
        wattages.sort();
        let max_scale_new = Math.max(wattages[wattages.length - 1], 1);
        set_max_scale(max_scale_new);
        setSelectedPlantOwners(
            {
                // add the new, but
                ...new_all,
                // override with existing setting
                ...selectedPlantOwners
            }
        )
    }, [props.power_plants]);


    const handlePlantTypeChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setPlantTypes({
            ...plantTypes,
            [event.target.name]: event.target.checked,
        })
    }

    const handleOwnersChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log(['owners-change-event'], event)
        const picked_owner = event.target.name;
        if (selectedPlantOwners[picked_owner]) {
            let copy = { ...selectedPlantOwners };
            copy[picked_owner] = false;
            setSelectedPlantOwners(copy);
        } else {
            let copy = { ...selectedPlantOwners };
            copy[picked_owner] = true;
            setSelectedPlantOwners(copy);
        }
    }



    return <div className='globe-container'>
        <div className='GlobeContent'>
            <Globe
                globeImageUrl={earth}
                width={800}
                height={600}
                htmlElementsData={selected_plants}
                htmlLat={(d: object) => (d as Plant).coordinates[0] / COORDINATE_PRECISION}
                htmlLng={(d: object) => (d as Plant).coordinates[1] / COORDINATE_PRECISION}
                htmlAltitude={0.02}
                htmlElement={(m: object) => {
                    const d: Plant = m as Plant;
                    const svg = d.plant_type === 'Hydro' ? hydroSvg : (d.plant_type === 'Flare' ? fireSvg : solarSvg);
                    const el = document.createElement('div');
                    el.innerHTML = svg;
                    el.style.color = 'white';
                    // can change size based on watts or hashrate
                    el.style.width = '50px';
                    // need this?
                    el.style.pointerEvents = 'auto';
                    el.style.cursor = 'pointer';
                    // set to 
                    el.onclick = () => PlantSelected(d.id);
                    return el;
                }}

                hexPolygonsData={countries_data.features}
                hexPolygonResolution={3}
                hexPolygonMargin={0.3}
                hexPolygonColor={(d: object) => {
                    type R = typeof countries_data.features[number];
                    let accessor: R = d as R;
                    return memoized_color(accessor.properties.NAME);
                }
                }
                onHexPolygonClick={(_polygon, _ev, { lat, lng }) => {
                    setLocation({ lat, lng })
                    console.log(['globe-click'], { lat, lng });
                    emit('globe-click', [lat, lng]);
                }}
                pointsData={output_bars}
                pointLabel={(d: object) => {
                    const p = d as BarData;
                    let label = `<></>`;
                    if (p.hashboards) {
                        label = `
                                <b>ID: ${p.id}</b> <br />
                                Hashboards: <i>${p.hashboards}</i> <br />
                                `
                    }
                    if (p.scale) {
                        label = `
                                <b>ID: ${p.id}</b> <br />
                                Scale: <i>${p.scale}</i> <br />
                                `
                    }
                    return label;
                }}
                pointLat={(d: object) => (d as BarData).coordinates[0] / COORDINATE_PRECISION}
                pointLng={(d: object) => (d as BarData).coordinates[1] / COORDINATE_PRECISION}
                pointAltitude={(d: object) => {
                    const p = d as BarData;
                    let alt = 0
                    console.log(["data-looks-like"], d);
                    if (p.hashboards) {
                        alt = p.hashboards
                    }
                    if (p.scale) {
                        alt = p.scale
                    }
                    return scaling_formula(alt, max_scale)
                }}
                pointRadius={0.15}
                pointColor={(d: object) => {
                    console.log(["color-data-shape"], d);
                    return chose_color(d as BarData)
                }}
                pointResolution={12}
                pointsMerge={true}
            />
        </div>
        <Typography>
            {location ? `Selected Location: ${location.lat}, ${location.lng}` : 'Click to select location'}
        </Typography>
        <Divider />
        <div className="GlobeToggles">
        <PlantTypeSelect handleChange={handlePlantTypeChange} plantTypes={plantTypes} />
        <PlantOwnerSelect handleChange={handleOwnersChange} plantOwners={all_plant_owners} selectedPlantOwners={selectedPlantOwners} user_id={props.user_id} />
        </div>
    </div >;
};

