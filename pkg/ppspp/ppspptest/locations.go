// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"github.com/golang/geo/s2"
)

// City ...
type City struct {
	LatLng s2.LatLng
	Name   string
}

// cities
var (
	Tokyo           = City{s2.LatLngFromDegrees(35.6850, 139.7514), "Tokyo"}
	NewYork         = City{s2.LatLngFromDegrees(40.6943, -73.9249), "New York"}
	MexicoCity      = City{s2.LatLngFromDegrees(19.4424, -99.1310), "Mexico City"}
	Mumbai          = City{s2.LatLngFromDegrees(19.0170, 72.8570), "Mumbai"}
	SaoPaulo        = City{s2.LatLngFromDegrees(-23.5587, -46.6250), "Sao Paulo"}
	Delhi           = City{s2.LatLngFromDegrees(28.6700, 77.2300), "Delhi"}
	Shanghai        = City{s2.LatLngFromDegrees(31.2165, 121.4365), "Shanghai"}
	Kolkata         = City{s2.LatLngFromDegrees(22.4950, 88.3247), "Kolkata"}
	LosAngeles      = City{s2.LatLngFromDegrees(34.1139, -118.4068), "Los Angeles"}
	Dhaka           = City{s2.LatLngFromDegrees(23.7231, 90.4086), "Dhaka"}
	BuenosAires     = City{s2.LatLngFromDegrees(-34.6025, -58.3975), "Buenos Aires"}
	Karachi         = City{s2.LatLngFromDegrees(24.8700, 66.9900), "Karachi"}
	Cairo           = City{s2.LatLngFromDegrees(30.0500, 31.2500), "Cairo"}
	RiodeJaneiro    = City{s2.LatLngFromDegrees(-22.9250, -43.2250), "Riode Janeiro"}
	Osaka           = City{s2.LatLngFromDegrees(34.7500, 135.4601), "Osaka"}
	Beijing         = City{s2.LatLngFromDegrees(39.9289, 116.3883), "Beijing"}
	Manila          = City{s2.LatLngFromDegrees(14.6042, 120.9822), "Manila"}
	Moscow          = City{s2.LatLngFromDegrees(55.7522, 37.6155), "Moscow"}
	Istanbul        = City{s2.LatLngFromDegrees(41.1050, 29.0100), "Istanbul"}
	Paris           = City{s2.LatLngFromDegrees(48.8667, 2.3333), "Paris"}
	Seoul           = City{s2.LatLngFromDegrees(37.5663, 126.9997), "Seoul"}
	Lagos           = City{s2.LatLngFromDegrees(6.4433, 3.3915), "Lagos"}
	Jakarta         = City{s2.LatLngFromDegrees(-6.1744, 106.8294), "Jakarta"}
	Guangzhou       = City{s2.LatLngFromDegrees(23.1450, 113.3250), "Guangzhou"}
	Chicago         = City{s2.LatLngFromDegrees(41.8373, -87.6862), "Chicago"}
	London          = City{s2.LatLngFromDegrees(51.5000, -0.1167), "London"}
	Lima            = City{s2.LatLngFromDegrees(-12.0480, -77.0501), "Lima"}
	Tehran          = City{s2.LatLngFromDegrees(35.6719, 51.4243), "Tehran"}
	Kinshasa        = City{s2.LatLngFromDegrees(-4.3297, 15.3150), "Kinshasa"}
	Bogota          = City{s2.LatLngFromDegrees(4.5964, -74.0833), "Bogota"}
	Shenzhen        = City{s2.LatLngFromDegrees(22.5524, 114.1221), "Shenzhen"}
	Wuhan           = City{s2.LatLngFromDegrees(30.5800, 114.2700), "Wuhan"}
	HongKong        = City{s2.LatLngFromDegrees(22.3050, 114.1850), "Hong Kong"}
	Tianjin         = City{s2.LatLngFromDegrees(39.1300, 117.2000), "Tianjin"}
	Chennai         = City{s2.LatLngFromDegrees(13.0900, 80.2800), "Chennai"}
	Taipei          = City{s2.LatLngFromDegrees(25.0358, 121.5683), "Taipei"}
	Bengaluru       = City{s2.LatLngFromDegrees(12.9700, 77.5600), "Bengaluru"}
	Bangkok         = City{s2.LatLngFromDegrees(13.7500, 100.5166), "Bangkok"}
	Lahore          = City{s2.LatLngFromDegrees(31.5600, 74.3500), "Lahore"}
	Chongqing       = City{s2.LatLngFromDegrees(29.5650, 106.5950), "Chongqing"}
	Miami           = City{s2.LatLngFromDegrees(25.7839, -80.2102), "Miami"}
	Hyderabad       = City{s2.LatLngFromDegrees(17.4000, 78.4800), "Hyderabad"}
	Dallas          = City{s2.LatLngFromDegrees(32.7936, -96.7662), "Dallas"}
	Santiago        = City{s2.LatLngFromDegrees(-33.4500, -70.6670), "Santiago"}
	Philadelphia    = City{s2.LatLngFromDegrees(40.0077, -75.1339), "Philadelphia"}
	BeloHorizonte   = City{s2.LatLngFromDegrees(-19.9150, -43.9150), "Belo Horizonte"}
	Madrid          = City{s2.LatLngFromDegrees(40.4000, -3.6834), "Madrid"}
	Houston         = City{s2.LatLngFromDegrees(29.7869, -95.3905), "Houston"}
	Ahmadabad       = City{s2.LatLngFromDegrees(23.0301, 72.5800), "Ahmadabad"}
	HoChiMinhCity   = City{s2.LatLngFromDegrees(10.7800, 106.6950), "Ho Chi MinhCity"}
	Washington      = City{s2.LatLngFromDegrees(38.9047, -77.0163), "Washington"}
	Atlanta         = City{s2.LatLngFromDegrees(33.7627, -84.4225), "Atlanta"}
	Toronto         = City{s2.LatLngFromDegrees(43.7000, -79.4200), "Toronto"}
	Singapore       = City{s2.LatLngFromDegrees(1.2930, 103.8558), "Singapore"}
	Luanda          = City{s2.LatLngFromDegrees(-8.8383, 13.2344), "Luanda"}
	Baghdad         = City{s2.LatLngFromDegrees(33.3386, 44.3939), "Baghdad"}
	Barcelona       = City{s2.LatLngFromDegrees(41.3833, 2.1834), "Barcelona"}
	Haora           = City{s2.LatLngFromDegrees(22.5804, 88.3299), "Haora"}
	Shenyang        = City{s2.LatLngFromDegrees(41.8050, 123.4500), "Shenyang"}
	Khartoum        = City{s2.LatLngFromDegrees(15.5881, 32.5342), "Khartoum"}
	Pune            = City{s2.LatLngFromDegrees(18.5300, 73.8500), "Pune"}
	Boston          = City{s2.LatLngFromDegrees(42.3188, -71.0846), "Boston"}
	Sydney          = City{s2.LatLngFromDegrees(-33.9200, 151.1852), "Sydney"}
	SaintPetersburg = City{s2.LatLngFromDegrees(59.9390, 30.3160), "Saint Petersburg"}
	Chittagong      = City{s2.LatLngFromDegrees(22.3300, 91.8000), "Chittagong"}
	Dongguan        = City{s2.LatLngFromDegrees(23.0489, 113.7447), "Dongguan"}
	Riyadh          = City{s2.LatLngFromDegrees(24.6408, 46.7727), "Riyadh"}
	Hanoi           = City{s2.LatLngFromDegrees(21.0333, 105.8500), "Hanoi"}
	Guadalajara     = City{s2.LatLngFromDegrees(20.6700, -103.3300), "Guadalajara"}
	Melbourne       = City{s2.LatLngFromDegrees(-37.8200, 144.9750), "Melbourne"}
	Alexandria      = City{s2.LatLngFromDegrees(31.2000, 29.9500), "Alexandria"}
	Chengdu         = City{s2.LatLngFromDegrees(30.6700, 104.0700), "Chengdu"}
	Rangoon         = City{s2.LatLngFromDegrees(16.7834, 96.1667), "Rangoon"}
	Phoenix         = City{s2.LatLngFromDegrees(33.5722, -112.0891), "Phoenix"}
	Xian            = City{s2.LatLngFromDegrees(34.2750, 108.8950), "Xian"}
	PortoAlegre     = City{s2.LatLngFromDegrees(-30.0500, -51.2000), "Porto Alegre"}
	Surat           = City{s2.LatLngFromDegrees(21.2000, 72.8400), "Surat"}
	Hechi           = City{s2.LatLngFromDegrees(23.0965, 109.6091), "Hechi"}
	Abidjan         = City{s2.LatLngFromDegrees(5.3200, -4.0400), "Abidjan"}
	Brasilia        = City{s2.LatLngFromDegrees(-15.7833, -47.9161), "Brasilia"}
	Ankara          = City{s2.LatLngFromDegrees(39.9272, 32.8644), "Ankara"}
	Monterrey       = City{s2.LatLngFromDegrees(25.6700, -100.3300), "Monterrey"}
	Yokohama        = City{s2.LatLngFromDegrees(35.3200, 139.5800), "Yokohama"}
	Nanjing         = City{s2.LatLngFromDegrees(32.0500, 118.7800), "Nanjing"}
	Montreal        = City{s2.LatLngFromDegrees(45.5000, -73.5833), "Montreal"}
	Guiyang         = City{s2.LatLngFromDegrees(26.5800, 106.7200), "Guiyang"}
	Recife          = City{s2.LatLngFromDegrees(-8.0756, -34.9156), "Recife"}
	Seattle         = City{s2.LatLngFromDegrees(47.6211, -122.3244), "Seattle"}
	Harbin          = City{s2.LatLngFromDegrees(45.7500, 126.6500), "Harbin"}
	SanFrancisco    = City{s2.LatLngFromDegrees(37.7562, -122.4430), "San Francisco"}
	Fortaleza       = City{s2.LatLngFromDegrees(-3.7500, -38.5800), "Fortaleza"}
	Zhangzhou       = City{s2.LatLngFromDegrees(24.5204, 117.6700), "Zhangzhou"}
	Detroit         = City{s2.LatLngFromDegrees(42.3834, -83.1024), "Detroit"}
	Salvador        = City{s2.LatLngFromDegrees(-12.9700, -38.4800), "Salvador"}
	Busan           = City{s2.LatLngFromDegrees(35.0951, 129.0100), "Busan"}
	Johannesburg    = City{s2.LatLngFromDegrees(-26.1700, 28.0300), "Johannesburg"}
	Berlin          = City{s2.LatLngFromDegrees(52.5218, 13.4015), "Berlin"}
	Algiers         = City{s2.LatLngFromDegrees(36.7631, 3.0506), "Algiers"}
	Rome            = City{s2.LatLngFromDegrees(41.8960, 12.4833), "Rome"}
)
