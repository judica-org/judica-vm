import { appWindow } from '@tauri-apps/api/window';
import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent, Icon } from '@mui/material';
import { emit } from '@tauri-apps/api/event';
import { fireSvg, solarSvg, hydroSvg } from './util';
import { Key } from '@mui/icons-material';
const { useState, useEffect } = React;

const stub_plant_data = [{
    coordinates: [46.818188, 8.227512],
    for_sale: false,
    hashrate: 90,
    id: 45637,
    miners: 32,
    owner: 12345566,
    plant_type: "Flare",
    text: "Flare Plant",
    watts: 345634,
}, {
    coordinates: [49.817492, 15.472962],
    for_sale: false,
    hashrate: 327,
    id: 13436,
    miners: 206,
    owner: 12345566,
    plant_type: "Hydro",
    text: "Hydroelectric Plant",
    watts: 67897,
}, {
    coordinates: [9.145, 40.489673],
    for_sale: true,
    hashrate: 141,
    id: 95944,
    miners: 30,
    owner: 12345566,
    plant_type: "Solar",
    text: "Solar Farm",
    watts: 23795,
}];

export default () => {
    const [power_plants, set_power_plants] = useState([]); // use empty list for now so it will render
    const [countries, setCountries] = useState([]);
    const [location, setLocation] = useState(null);
    const [selected_plant, setSelectedPlant] = useState(null);

    useEffect(() => {
        setCountries(countries_data);
        const unlisten_power_plants = appWindow.listen("power-plants", (ev) => {
            console.log(['game-board-event'], ev);
            set_power_plants(ev.payload)
        });

        return () => {
            (async () => {
                (await unlisten_power_plants)();
            })();
        }
    }, [power_plants, location]);

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
                        htmlElementsData={power_plants}
                        htmlLat={d => d.coordinates[0]}
                        htmlLng={d => d.coordinates[1]}
                        htmlAltitude={0.02}
                        htmlElement={d => {
                            const svg = d.plant_type === 'Hydro' ? hydroSvg : (d.plant_type === 'Flare' ? fireSvg : solarSvg);
                            const el = document.createElement('div');
                            el.innerHTML = svg;
                            el.style.color = 'white';
                            // can change size based on watts or hashrate
                            el.style.width = '50px';
                            // need this?
                            el.style['pointer-events'] = 'auto';
                            el.style.cursor = 'pointer';
                            // set to 
                            el.onclick = () => setSelectedPlant(d);
                            el.onmouseover = () => {
                                el.innerHTML = `
                                <b>ID: ${d.id}</b> <br />
                                Owner: <i>${d.owner}</i> <br />
                                Watts: <i>${d.watts}</i> <br />
                                ${d.for_sale ? 'For Sale' : ''}
                                `
                            }
                            el.onmouseleave = () => el.innerHTML = svg;
                            return el;
                        }}

                        hexPolygonsData={countries.features}
                        hexPolygonResolution={3}
                        hexPolygonMargin={0.3}
                        hexPolygonColor={
                            () => `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`
                        }
                        onHexPolygonClick={(_polygon, _ev, { lat, lng }) => {
                            setLocation({ lat, lng })
                            console.log(['globe-click'], { lat, lng });
                            emit('globe-click', [lat, lng]);
                        }}
                    />
                </div>
            </CardContent>
        </Card>
    </div>;
};