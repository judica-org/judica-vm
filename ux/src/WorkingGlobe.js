import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent } from '@mui/material';
const { useState, useEffect } = React;

const plant_data = [{
    id: 45637,
    lat: 46.818188,
    lng: 8.227512,
    plant_type: "nuclear",
    text: "Nuclear Plant",
    owner: 12345566,
    watts: 345634,
    for_sale: false
}, {
    id: 13436,
    lat: 49.817492,
    lng: 15.472962,
    plant_type: "wind",
    text: "Wind Plant",
    owner: 12345566,
    watts: 67897,
    for_sale: false
}, {
    id: 95944,
    lat: 9.145,
    lng: 40.489673,
    plant_type: "solar",
    text: "Solar Farm",
    owner: 12345566,
    watts: 23795,
    for_sale: true
}]

// eslint-disable-next-line import/no-anonymous-default-export
export default ({ power_plants }) => {
    const [countries, setCountries] = useState({ features: [] });
    const [plants, setPlants] = useState([]);
    useEffect(() => {
        // load data
        setCountries(countries_data);
        setPlants(plant_data);
    }, [plants]);

    return <div className='globe-container'>
        <Card>
            <CardHeader title={'Map'}
                subheader={'Current Energy Grid'}
            />
            <CardContent className={'content'} style={{
                position: 'relative'
            }}>
                <Globe
                    globeImageUrl={earth}
                    width={500}
                    height={500}
                    labelsData={plants}
                    labelText={'text'}
                    labelSize={2}
                    labelColor={() => 'white'}
                    labelAltitude={0.1}
                    labelIncludeDot={true}
                    labelDotRadius={0.5}
                    labelDotOrientation={() => 'top'}
                    labelLabel={
                        (l) => `
                            <b>ID: ${l.id}</b> <br />
                            Owner: <i>${l.owner}</i> <br />
                            Watts: <i>${l.watts}</i> <br />
                            ${l.for_sale ? 'For Sale' : ''}
                            `
                    }
                    hexPolygonsData={countries.features}
                    hexPolygonResolution={3}
                    hexPolygonMargin={0.3}
                    hexPolygonColor={
                        () => `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`
                    }
                    hexPolygonLabel={
                        ({ properties: d }) => `
        <b>${d.ADMIN} (${d.ISO_A2})</b> <br />
        Population: <i>${d.POP_EST}</i>
      `
                    }
                />
            </CardContent>
        </Card>
    </div>;
};