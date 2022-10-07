import { appWindow } from '@tauri-apps/api/window';
import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent, Icon, Divider } from '@mui/material';
import { emit, Event } from '@tauri-apps/api/event';
import { fireSvg, solarSvg, hydroSvg } from './util';
import { PlantSelected, PowerPlant, UserPowerPlant } from './App';
import { PlantOwnerSelect, PlantTypeSelect } from './GlobeHelpers';
import { COORDINATE_PRECISION } from './mint-power-plant/MintingForm';
import { EntityID } from './Types/GameMove';
const { useState, useEffect } = React;

type Plant = (UserPowerPlant & { text: string });
const stub_plant_data: Plant[] = [{
    coordinates: [46818188, 8227512],
    for_sale: false,
    hashrate: 90000,
    id: "45637",
    miners: 32,
    owner: "12345566",
    plant_type: "Flare",
    text: "Flare Plant",
    watts: 345634,
}, {
    coordinates: [49817492, 15472962],
    for_sale: false,
    hashrate: 32700,
    id: "13436",
    miners: 206,
    owner: "9494384",
    plant_type: "Hydro",
    text: "Hydroelectric Plant",
    watts: 67897,
}, {
    coordinates: [9145125, 40489673],
    for_sale: true,
    hashrate: 14100,
    id: "95944",
    miners: 30,
    owner: "12345566",
    plant_type: "Solar",
    text: "Solar Farm",
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

function getBarData(plants: (UserPowerPlant & { text: string })[]) {
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

export default () => {
    const [power_plants, set_power_plants] = useState<(UserPowerPlant & { text: string })[]>([]); // use empty list for now so it will render
    const [owners, setOwners] = useState<EntityID[]>([]);
    const [selectedPlantOwners, setSelectedPlantOwners] = useState<EntityID[]>([]); // default to all owners
    const [output_bars, set_output_bars] = useState<BarData[]>([]);
    const [location, setLocation] = useState<{ lat: number, lng: number } | null>(null);
    const [plantTypes, setPlantTypes] = React.useState({
        'Hydro': true,
        'Solar': true,
        'Flare': true
    });

    const handlePlantTypeChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setPlantTypes({
            ...plantTypes,
            [event.target.name]: event.target.checked,
        })
    }

    const handleOwnersChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log(['owners-change-event'], event)
        const picked_owner = event.target.name;
        if (selectedPlantOwners.includes(picked_owner)) {
            setSelectedPlantOwners(selectedPlantOwners.filter((owner) => owner !== picked_owner));
        } else {
            setSelectedPlantOwners([...selectedPlantOwners, picked_owner]);
        }
    }

    useEffect(() => {
        const unlisten_power_plants = appWindow.listen("power-plants", (ev: Event<(UserPowerPlant & { text: string })[]>) => {
            console.log(['power-plants-received'], ev);
            let plant_owners: string[] = [];
            ev.payload.forEach((plant) => {
                if (!plant_owners.includes(plant.owner)) {
                    plant_owners.push(plant.owner)
                }
            })
            setOwners(plant_owners);
            setSelectedPlantOwners(plant_owners)
            set_power_plants(ev.payload);
            set_output_bars(getBarData(ev.payload));
        });

        return () => {
            (async () => {
                (await unlisten_power_plants)();
            })();
        }
    }, [power_plants, owners, location]);

    const selectedPlantTypes = Object.entries(plantTypes).filter(([_type, selected]) => selected === true).map(([type, _selected]) => type);
    const plants_by_type = power_plants.filter(({ plant_type, owner }) => selectedPlantTypes.includes(plant_type) && selectedPlantOwners.includes(owner));

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
                        htmlElementsData={plants_by_type}
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

