import { appWindow } from '@tauri-apps/api/window';
import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent, Icon, Divider } from '@mui/material';
import { emit, Event } from '@tauri-apps/api/event';
import { fireSvg, solarSvg, hydroSvg } from './util';
import { PlantSelected, PlantType } from './App';
import { PlantOwnerSelect, PlantTypeSelect } from './GlobeHelpers';
import { COORDINATE_PRECISION } from './mint-power-plant/MintingForm';
import { EntityID } from './Types/GameMove';
import { UXPlantData } from './Types/Gameboard';
const { useState, useEffect } = React;

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

type BarData = { coordinates: number[], hashrate?: number, watts?: number, id: string };

function getBarData(plants: (UXPlantData)[]) {
    return plants.reduce<BarData[]>((acc, { coordinates, hashrate, watts, id }) => {
        return [...acc, { id, coordinates, hashrate }, { id, coordinates: [coordinates[0] + 100000, coordinates[1] + 100000], watts }];
    }, []);
}

const chose_color = (d: BarData): string => {
    console.log(["picking-color"], d);
    if (d.hashrate && d.hashrate > 0) {
        return 'orange'
    } else {
        return 'green'
    }
}

export default (props: { power_plants: UXPlantData[] }) => {
    console.log("POWER FOR GLOBE", props.power_plants);
    let plant_owners: Set<string> = new Set();
    props.power_plants.forEach((plant) => {
        plant_owners.add(plant.owner)
    })
    const owners = Array.from(plant_owners.entries()).map(([a, b]) => a);
    const output_bars = getBarData(props.power_plants);
    const [selectedPlantOwners, setSelectedPlantOwners] = useState<Record<EntityID, null>>(Object.fromEntries(owners.map((a) => [a, null]))); // default to all owners
    const [location, setLocation] = useState<{ lat: number, lng: number } | null>(null);
    const [plantTypes, setPlantTypes] = React.useState<Record<PlantType, boolean>>({
        'Hydro': true,
        'Solar': true,
        'Flare': true
    });

    const [selected_plants, set_selected_plants] = React.useState<UXPlantData[]>(props.power_plants);


    React.useEffect(() => {
        const plants_by_type = props.power_plants.filter(({ plant_type, owner }) => plantTypes[plant_type] && Object.hasOwn(selectedPlantOwners, owner));
        set_selected_plants(plants_by_type);
    }, [plantTypes, selectedPlantOwners]);


    const handlePlantTypeChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setPlantTypes({
            ...plantTypes,
            [event.target.name]: event.target.checked,
        })
    }

    const handleOwnersChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log(['owners-change-event'], event)
        const picked_owner = event.target.name;
        if (Object.hasOwn(selectedPlantOwners, picked_owner)) {
            let copy = { ...selectedPlantOwners };
            delete copy[picked_owner];
            setSelectedPlantOwners(copy);
        } else {
            let e = Object.entries(selectedPlantOwners);
            e.push([picked_owner, null]);
            let copy = Object.fromEntries(e);
            setSelectedPlantOwners(copy);
        }
    }



    return <div className='globe-container'>
        <Card>
            <CardHeader title={'World Energy Grid'}
                subheader={location ? `Selected Location: ${location.lat}, ${location.lng}` : 'Click to select location'}
            />
            <CardContent  >
                <div className='GlobeContent'>
                    <Globe
                        globeImageUrl={earth}
                        width={600}
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
                            el.onmouseover = () => {
                                el.innerHTML = `
                                <b>ID: ${d.id}</b> <br />
                                Owner: <i>${d.owner}</i> <br />
                                Watts: <i>${d.watts}</i> <br />
                                Hashrate: <i>${d.hashrate}</i> <br />
                                ${d.for_sale ? 'For Sale' : ''}
                                `
                            }
                            el.onmouseleave = () => el.innerHTML = svg;
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
                            if (p.hashrate) {
                                label = `
                                <b>ID: ${p.id}</b> <br />
                                Hashrate: <i>${p.hashrate}</i> <br />
                                `
                            }
                            if (p.watts) {
                                label = `
                                <b>ID: ${p.id}</b> <br />
                                Watts: <i>${p.watts}</i> <br />
                                `
                            }
                            return label;
                        }}
                        pointLat={(d: object) => (d as BarData).coordinates[0] / COORDINATE_PRECISION}
                        pointLng={(d: object) => (d as BarData).coordinates[1] / COORDINATE_PRECISION}
                        pointAltitude={(d: object) => {
                            const p = d as BarData;
                            let alt = 1
                            console.log(["data-looks-like"], d);
                            if (p.hashrate) {
                                alt = p.hashrate! * 6e-6
                            }
                            if (p.watts) {
                                alt = p.watts! * 6e-6
                            }
                            return alt
                        }}
                        pointRadius={0.25}
                        pointColor={(d: object) => chose_color(d as BarData)}
                        pointResolution={12}
                        pointsMerge={true}
                    />
                </div>
                <Divider />
                <PlantTypeSelect handleChange={handlePlantTypeChange} plantTypes={plantTypes} />
                <PlantOwnerSelect handleChange={handleOwnersChange} plantOwners={owners} selectedPlantOwners={selectedPlantOwners} />
            </CardContent>
        </Card>
    </div>;
};

