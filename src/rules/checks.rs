use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use rustpython_ast::Location;

use crate::ast::types::Range;
use crate::autofix::Fix;
use crate::checks::CheckKind;
use crate::source_code_locator::SourceCodeLocator;
use crate::Check;

/// See: https://github.com/microsoft/vscode/blob/095ddabc52b82498ee7f718a34f9dd11d59099a8/src/vs/base/common/strings.ts#L1195
static CONFUSABLES: Lazy<BTreeMap<u32, u32>> = Lazy::new(|| {
    BTreeMap::from([
        (8232, 32),
        (8233, 32),
        (5760, 32),
        (8192, 32),
        (8193, 32),
        (8194, 32),
        (8195, 32),
        (8196, 32),
        (8197, 32),
        (8198, 32),
        (8200, 32),
        (8201, 32),
        (8202, 32),
        (8287, 32),
        (8199, 32),
        (8239, 32),
        (2042, 95),
        (65101, 95),
        (65102, 95),
        (65103, 95),
        (8208, 45),
        (8209, 45),
        (8210, 45),
        (65112, 45),
        (1748, 45),
        (8259, 45),
        (727, 45),
        (8722, 45),
        (10134, 45),
        (11450, 45),
        (1549, 44),
        (1643, 44),
        (8218, 44),
        (184, 44),
        (42233, 44),
        (894, 59),
        (2307, 58),
        (2691, 58),
        (1417, 58),
        (1795, 58),
        (1796, 58),
        (5868, 58),
        (65072, 58),
        (6147, 58),
        (6153, 58),
        (8282, 58),
        (1475, 58),
        (760, 58),
        (42889, 58),
        (8758, 58),
        (720, 58),
        (42237, 58),
        (451, 33),
        (11601, 33),
        (660, 63),
        (577, 63),
        (2429, 63),
        (5038, 63),
        (42731, 63),
        (119149, 46),
        (8228, 46),
        (1793, 46),
        (1794, 46),
        (42510, 46),
        (68176, 46),
        (1632, 46),
        (1776, 46),
        (42232, 46),
        (1373, 96),
        (65287, 96),
        (8219, 96),
        (8242, 96),
        (1370, 96),
        (1523, 96),
        (8175, 96),
        (65344, 96),
        (900, 96),
        (8189, 96),
        (8125, 96),
        (8127, 96),
        (8190, 96),
        (697, 96),
        (884, 96),
        (712, 96),
        (714, 96),
        (715, 96),
        (756, 96),
        (699, 96),
        (701, 96),
        (700, 96),
        (702, 96),
        (42892, 96),
        (1497, 96),
        (2036, 96),
        (2037, 96),
        (5194, 96),
        (5836, 96),
        (94033, 96),
        (94034, 96),
        (65339, 91),
        (10088, 40),
        (10098, 40),
        (12308, 40),
        (64830, 40),
        (65341, 93),
        (10089, 41),
        (10099, 41),
        (12309, 41),
        (64831, 41),
        (10100, 123),
        (119060, 123),
        (10101, 125),
        (65342, 94),
        (8270, 42),
        (1645, 42),
        (8727, 42),
        (66335, 42),
        (5941, 47),
        (8257, 47),
        (8725, 47),
        (8260, 47),
        (9585, 47),
        (10187, 47),
        (10744, 47),
        (119354, 47),
        (12755, 47),
        (12339, 47),
        (11462, 47),
        (20031, 47),
        (12035, 47),
        (65340, 92),
        (65128, 92),
        (8726, 92),
        (10189, 92),
        (10741, 92),
        (10745, 92),
        (119311, 92),
        (119355, 92),
        (12756, 92),
        (20022, 92),
        (12034, 92),
        (42872, 38),
        (708, 94),
        (710, 94),
        (5869, 43),
        (10133, 43),
        (66203, 43),
        (8249, 60),
        (10094, 60),
        (706, 60),
        (119350, 60),
        (5176, 60),
        (5810, 60),
        (5120, 61),
        (11840, 61),
        (12448, 61),
        (42239, 61),
        (8250, 62),
        (10095, 62),
        (707, 62),
        (119351, 62),
        (5171, 62),
        (94015, 62),
        (8275, 126),
        (732, 126),
        (8128, 126),
        (8764, 126),
        (65372, 124),
        (65293, 45),
        (120784, 50),
        (120794, 50),
        (120804, 50),
        (120814, 50),
        (120824, 50),
        (130034, 50),
        (42842, 50),
        (423, 50),
        (1000, 50),
        (42564, 50),
        (5311, 50),
        (42735, 50),
        (119302, 51),
        (120785, 51),
        (120795, 51),
        (120805, 51),
        (120815, 51),
        (120825, 51),
        (130035, 51),
        (42923, 51),
        (540, 51),
        (439, 51),
        (42858, 51),
        (11468, 51),
        (1248, 51),
        (94011, 51),
        (71882, 51),
        (120786, 52),
        (120796, 52),
        (120806, 52),
        (120816, 52),
        (120826, 52),
        (130036, 52),
        (5070, 52),
        (71855, 52),
        (120787, 53),
        (120797, 53),
        (120807, 53),
        (120817, 53),
        (120827, 53),
        (130037, 53),
        (444, 53),
        (71867, 53),
        (120788, 54),
        (120798, 54),
        (120808, 54),
        (120818, 54),
        (120828, 54),
        (130038, 54),
        (11474, 54),
        (5102, 54),
        (71893, 54),
        (119314, 55),
        (120789, 55),
        (120799, 55),
        (120809, 55),
        (120819, 55),
        (120829, 55),
        (130039, 55),
        (66770, 55),
        (71878, 55),
        (2819, 56),
        (2538, 56),
        (2666, 56),
        (125131, 56),
        (120790, 56),
        (120800, 56),
        (120810, 56),
        (120820, 56),
        (120830, 56),
        (130040, 56),
        (547, 56),
        (546, 56),
        (66330, 56),
        (2663, 57),
        (2920, 57),
        (2541, 57),
        (3437, 57),
        (120791, 57),
        (120801, 57),
        (120811, 57),
        (120821, 57),
        (120831, 57),
        (130041, 57),
        (42862, 57),
        (11466, 57),
        (71884, 57),
        (71852, 57),
        (71894, 57),
        (9082, 97),
        (65345, 97),
        (119834, 97),
        (119886, 97),
        (119938, 97),
        (119990, 97),
        (120042, 97),
        (120094, 97),
        (120146, 97),
        (120198, 97),
        (120250, 97),
        (120302, 97),
        (120354, 97),
        (120406, 97),
        (120458, 97),
        (593, 97),
        (945, 97),
        (120514, 97),
        (120572, 97),
        (120630, 97),
        (120688, 97),
        (120746, 97),
        (65313, 65),
        (119808, 65),
        (119860, 65),
        (119912, 65),
        (119964, 65),
        (120016, 65),
        (120068, 65),
        (120120, 65),
        (120172, 65),
        (120224, 65),
        (120276, 65),
        (120328, 65),
        (120380, 65),
        (120432, 65),
        (913, 65),
        (120488, 65),
        (120546, 65),
        (120604, 65),
        (120662, 65),
        (120720, 65),
        (5034, 65),
        (5573, 65),
        (42222, 65),
        (94016, 65),
        (66208, 65),
        (119835, 98),
        (119887, 98),
        (119939, 98),
        (119991, 98),
        (120043, 98),
        (120095, 98),
        (120147, 98),
        (120199, 98),
        (120251, 98),
        (120303, 98),
        (120355, 98),
        (120407, 98),
        (120459, 98),
        (388, 98),
        (5071, 98),
        (5234, 98),
        (5551, 98),
        (65314, 66),
        (8492, 66),
        (119809, 66),
        (119861, 66),
        (119913, 66),
        (120017, 66),
        (120069, 66),
        (120121, 66),
        (120173, 66),
        (120225, 66),
        (120277, 66),
        (120329, 66),
        (120381, 66),
        (120433, 66),
        (42932, 66),
        (914, 66),
        (120489, 66),
        (120547, 66),
        (120605, 66),
        (120663, 66),
        (120721, 66),
        (5108, 66),
        (5623, 66),
        (42192, 66),
        (66178, 66),
        (66209, 66),
        (66305, 66),
        (65347, 99),
        (8573, 99),
        (119836, 99),
        (119888, 99),
        (119940, 99),
        (119992, 99),
        (120044, 99),
        (120096, 99),
        (120148, 99),
        (120200, 99),
        (120252, 99),
        (120304, 99),
        (120356, 99),
        (120408, 99),
        (120460, 99),
        (7428, 99),
        (1010, 99),
        (11429, 99),
        (43951, 99),
        (66621, 99),
        (128844, 67),
        (71922, 67),
        (71913, 67),
        (65315, 67),
        (8557, 67),
        (8450, 67),
        (8493, 67),
        (119810, 67),
        (119862, 67),
        (119914, 67),
        (119966, 67),
        (120018, 67),
        (120174, 67),
        (120226, 67),
        (120278, 67),
        (120330, 67),
        (120382, 67),
        (120434, 67),
        (1017, 67),
        (11428, 67),
        (5087, 67),
        (42202, 67),
        (66210, 67),
        (66306, 67),
        (66581, 67),
        (66844, 67),
        (8574, 100),
        (8518, 100),
        (119837, 100),
        (119889, 100),
        (119941, 100),
        (119993, 100),
        (120045, 100),
        (120097, 100),
        (120149, 100),
        (120201, 100),
        (120253, 100),
        (120305, 100),
        (120357, 100),
        (120409, 100),
        (120461, 100),
        (1281, 100),
        (5095, 100),
        (5231, 100),
        (42194, 100),
        (8558, 68),
        (8517, 68),
        (119811, 68),
        (119863, 68),
        (119915, 68),
        (119967, 68),
        (120019, 68),
        (120071, 68),
        (120123, 68),
        (120175, 68),
        (120227, 68),
        (120279, 68),
        (120331, 68),
        (120383, 68),
        (120435, 68),
        (5024, 68),
        (5598, 68),
        (5610, 68),
        (42195, 68),
        (8494, 101),
        (65349, 101),
        (8495, 101),
        (8519, 101),
        (119838, 101),
        (119890, 101),
        (119942, 101),
        (120046, 101),
        (120098, 101),
        (120150, 101),
        (120202, 101),
        (120254, 101),
        (120306, 101),
        (120358, 101),
        (120410, 101),
        (120462, 101),
        (43826, 101),
        (1213, 101),
        (8959, 69),
        (65317, 69),
        (8496, 69),
        (119812, 69),
        (119864, 69),
        (119916, 69),
        (120020, 69),
        (120072, 69),
        (120124, 69),
        (120176, 69),
        (120228, 69),
        (120280, 69),
        (120332, 69),
        (120384, 69),
        (120436, 69),
        (917, 69),
        (120492, 69),
        (120550, 69),
        (120608, 69),
        (120666, 69),
        (120724, 69),
        (11577, 69),
        (5036, 69),
        (42224, 69),
        (71846, 69),
        (71854, 69),
        (66182, 69),
        (119839, 102),
        (119891, 102),
        (119943, 102),
        (119995, 102),
        (120047, 102),
        (120099, 102),
        (120151, 102),
        (120203, 102),
        (120255, 102),
        (120307, 102),
        (120359, 102),
        (120411, 102),
        (120463, 102),
        (43829, 102),
        (42905, 102),
        (383, 102),
        (7837, 102),
        (1412, 102),
        (119315, 70),
        (8497, 70),
        (119813, 70),
        (119865, 70),
        (119917, 70),
        (120021, 70),
        (120073, 70),
        (120125, 70),
        (120177, 70),
        (120229, 70),
        (120281, 70),
        (120333, 70),
        (120385, 70),
        (120437, 70),
        (42904, 70),
        (988, 70),
        (120778, 70),
        (5556, 70),
        (42205, 70),
        (71874, 70),
        (71842, 70),
        (66183, 70),
        (66213, 70),
        (66853, 70),
        (65351, 103),
        (8458, 103),
        (119840, 103),
        (119892, 103),
        (119944, 103),
        (120048, 103),
        (120100, 103),
        (120152, 103),
        (120204, 103),
        (120256, 103),
        (120308, 103),
        (120360, 103),
        (120412, 103),
        (120464, 103),
        (609, 103),
        (7555, 103),
        (397, 103),
        (1409, 103),
        (119814, 71),
        (119866, 71),
        (119918, 71),
        (119970, 71),
        (120022, 71),
        (120074, 71),
        (120126, 71),
        (120178, 71),
        (120230, 71),
        (120282, 71),
        (120334, 71),
        (120386, 71),
        (120438, 71),
        (1292, 71),
        (5056, 71),
        (5107, 71),
        (42198, 71),
        (65352, 104),
        (8462, 104),
        (119841, 104),
        (119945, 104),
        (119997, 104),
        (120049, 104),
        (120101, 104),
        (120153, 104),
        (120205, 104),
        (120257, 104),
        (120309, 104),
        (120361, 104),
        (120413, 104),
        (120465, 104),
        (1211, 104),
        (1392, 104),
        (5058, 104),
        (65320, 72),
        (8459, 72),
        (8460, 72),
        (8461, 72),
        (119815, 72),
        (119867, 72),
        (119919, 72),
        (120023, 72),
        (120179, 72),
        (120231, 72),
        (120283, 72),
        (120335, 72),
        (120387, 72),
        (120439, 72),
        (919, 72),
        (120494, 72),
        (120552, 72),
        (120610, 72),
        (120668, 72),
        (120726, 72),
        (11406, 72),
        (5051, 72),
        (5500, 72),
        (42215, 72),
        (66255, 72),
        (731, 105),
        (9075, 105),
        (65353, 105),
        (8560, 105),
        (8505, 105),
        (8520, 105),
        (119842, 105),
        (119894, 105),
        (119946, 105),
        (119998, 105),
        (120050, 105),
        (120102, 105),
        (120154, 105),
        (120206, 105),
        (120258, 105),
        (120310, 105),
        (120362, 105),
        (120414, 105),
        (120466, 105),
        (120484, 105),
        (618, 105),
        (617, 105),
        (953, 105),
        (8126, 105),
        (890, 105),
        (120522, 105),
        (120580, 105),
        (120638, 105),
        (120696, 105),
        (120754, 105),
        (1110, 105),
        (42567, 105),
        (1231, 105),
        (43893, 105),
        (5029, 105),
        (71875, 105),
        (65354, 106),
        (8521, 106),
        (119843, 106),
        (119895, 106),
        (119947, 106),
        (119999, 106),
        (120051, 106),
        (120103, 106),
        (120155, 106),
        (120207, 106),
        (120259, 106),
        (120311, 106),
        (120363, 106),
        (120415, 106),
        (120467, 106),
        (1011, 106),
        (1112, 106),
        (65322, 74),
        (119817, 74),
        (119869, 74),
        (119921, 74),
        (119973, 74),
        (120025, 74),
        (120077, 74),
        (120129, 74),
        (120181, 74),
        (120233, 74),
        (120285, 74),
        (120337, 74),
        (120389, 74),
        (120441, 74),
        (42930, 74),
        (895, 74),
        (1032, 74),
        (5035, 74),
        (5261, 74),
        (42201, 74),
        (119844, 107),
        (119896, 107),
        (119948, 107),
        (120000, 107),
        (120052, 107),
        (120104, 107),
        (120156, 107),
        (120208, 107),
        (120260, 107),
        (120312, 107),
        (120364, 107),
        (120416, 107),
        (120468, 107),
        (8490, 75),
        (65323, 75),
        (119818, 75),
        (119870, 75),
        (119922, 75),
        (119974, 75),
        (120026, 75),
        (120078, 75),
        (120130, 75),
        (120182, 75),
        (120234, 75),
        (120286, 75),
        (120338, 75),
        (120390, 75),
        (120442, 75),
        (922, 75),
        (120497, 75),
        (120555, 75),
        (120613, 75),
        (120671, 75),
        (120729, 75),
        (11412, 75),
        (5094, 75),
        (5845, 75),
        (42199, 75),
        (66840, 75),
        (1472, 108),
        (8739, 73),
        (9213, 73),
        (65512, 73),
        (1633, 108),
        (1777, 73),
        (66336, 108),
        (125127, 108),
        (120783, 73),
        (120793, 73),
        (120803, 73),
        (120813, 73),
        (120823, 73),
        (130033, 73),
        (65321, 73),
        (8544, 73),
        (8464, 73),
        (8465, 73),
        (119816, 73),
        (119868, 73),
        (119920, 73),
        (120024, 73),
        (120128, 73),
        (120180, 73),
        (120232, 73),
        (120284, 73),
        (120336, 73),
        (120388, 73),
        (120440, 73),
        (65356, 108),
        (8572, 73),
        (8467, 108),
        (119845, 108),
        (119897, 108),
        (119949, 108),
        (120001, 108),
        (120053, 108),
        (120105, 73),
        (120157, 73),
        (120209, 73),
        (120261, 73),
        (120313, 73),
        (120365, 73),
        (120417, 73),
        (120469, 73),
        (448, 73),
        (120496, 73),
        (120554, 73),
        (120612, 73),
        (120670, 73),
        (120728, 73),
        (11410, 73),
        (1030, 73),
        (1216, 73),
        (1493, 108),
        (1503, 108),
        (1575, 108),
        (126464, 108),
        (126592, 108),
        (65166, 108),
        (65165, 108),
        (1994, 108),
        (11599, 73),
        (5825, 73),
        (42226, 73),
        (93992, 73),
        (66186, 124),
        (66313, 124),
        (119338, 76),
        (8556, 76),
        (8466, 76),
        (119819, 76),
        (119871, 76),
        (119923, 76),
        (120027, 76),
        (120079, 76),
        (120131, 76),
        (120183, 76),
        (120235, 76),
        (120287, 76),
        (120339, 76),
        (120391, 76),
        (120443, 76),
        (11472, 76),
        (5086, 76),
        (5290, 76),
        (42209, 76),
        (93974, 76),
        (71843, 76),
        (71858, 76),
        (66587, 76),
        (66854, 76),
        (65325, 77),
        (8559, 77),
        (8499, 77),
        (119820, 77),
        (119872, 77),
        (119924, 77),
        (120028, 77),
        (120080, 77),
        (120132, 77),
        (120184, 77),
        (120236, 77),
        (120288, 77),
        (120340, 77),
        (120392, 77),
        (120444, 77),
        (924, 77),
        (120499, 77),
        (120557, 77),
        (120615, 77),
        (120673, 77),
        (120731, 77),
        (1018, 77),
        (11416, 77),
        (5047, 77),
        (5616, 77),
        (5846, 77),
        (42207, 77),
        (66224, 77),
        (66321, 77),
        (119847, 110),
        (119899, 110),
        (119951, 110),
        (120003, 110),
        (120055, 110),
        (120107, 110),
        (120159, 110),
        (120211, 110),
        (120263, 110),
        (120315, 110),
        (120367, 110),
        (120419, 110),
        (120471, 110),
        (1400, 110),
        (1404, 110),
        (65326, 78),
        (8469, 78),
        (119821, 78),
        (119873, 78),
        (119925, 78),
        (119977, 78),
        (120029, 78),
        (120081, 78),
        (120185, 78),
        (120237, 78),
        (120289, 78),
        (120341, 78),
        (120393, 78),
        (120445, 78),
        (925, 78),
        (120500, 78),
        (120558, 78),
        (120616, 78),
        (120674, 78),
        (120732, 78),
        (11418, 78),
        (42208, 78),
        (66835, 78),
        (3074, 111),
        (3202, 111),
        (3330, 111),
        (3458, 111),
        (2406, 111),
        (2662, 111),
        (2790, 111),
        (3046, 111),
        (3174, 111),
        (3302, 111),
        (3430, 111),
        (3664, 111),
        (3792, 111),
        (4160, 111),
        (1637, 111),
        (1781, 111),
        (65359, 111),
        (8500, 111),
        (119848, 111),
        (119900, 111),
        (119952, 111),
        (120056, 111),
        (120108, 111),
        (120160, 111),
        (120212, 111),
        (120264, 111),
        (120316, 111),
        (120368, 111),
        (120420, 111),
        (120472, 111),
        (7439, 111),
        (7441, 111),
        (43837, 111),
        (959, 111),
        (120528, 111),
        (120586, 111),
        (120644, 111),
        (120702, 111),
        (120760, 111),
        (963, 111),
        (120532, 111),
        (120590, 111),
        (120648, 111),
        (120706, 111),
        (120764, 111),
        (11423, 111),
        (4351, 111),
        (1413, 111),
        (1505, 111),
        (1607, 111),
        (126500, 111),
        (126564, 111),
        (126596, 111),
        (65259, 111),
        (65260, 111),
        (65258, 111),
        (65257, 111),
        (1726, 111),
        (64428, 111),
        (64429, 111),
        (64427, 111),
        (64426, 111),
        (1729, 111),
        (64424, 111),
        (64425, 111),
        (64423, 111),
        (64422, 111),
        (1749, 111),
        (3360, 111),
        (4125, 111),
        (66794, 111),
        (71880, 111),
        (71895, 111),
        (66604, 111),
        (1984, 79),
        (2534, 79),
        (2918, 79),
        (12295, 79),
        (70864, 79),
        (71904, 79),
        (120782, 79),
        (120792, 79),
        (120802, 79),
        (120812, 79),
        (120822, 79),
        (130032, 79),
        (65327, 79),
        (119822, 79),
        (119874, 79),
        (119926, 79),
        (119978, 79),
        (120030, 79),
        (120082, 79),
        (120134, 79),
        (120186, 79),
        (120238, 79),
        (120290, 79),
        (120342, 79),
        (120394, 79),
        (120446, 79),
        (927, 79),
        (120502, 79),
        (120560, 79),
        (120618, 79),
        (120676, 79),
        (120734, 79),
        (11422, 79),
        (1365, 79),
        (11604, 79),
        (4816, 79),
        (2848, 79),
        (66754, 79),
        (42227, 79),
        (71861, 79),
        (66194, 79),
        (66219, 79),
        (66564, 79),
        (66838, 79),
        (9076, 112),
        (65360, 112),
        (119849, 112),
        (119901, 112),
        (119953, 112),
        (120005, 112),
        (120057, 112),
        (120109, 112),
        (120161, 112),
        (120213, 112),
        (120265, 112),
        (120317, 112),
        (120369, 112),
        (120421, 112),
        (120473, 112),
        (961, 112),
        (120530, 112),
        (120544, 112),
        (120588, 112),
        (120602, 112),
        (120646, 112),
        (120660, 112),
        (120704, 112),
        (120718, 112),
        (120762, 112),
        (120776, 112),
        (11427, 112),
        (65328, 80),
        (8473, 80),
        (119823, 80),
        (119875, 80),
        (119927, 80),
        (119979, 80),
        (120031, 80),
        (120083, 80),
        (120187, 80),
        (120239, 80),
        (120291, 80),
        (120343, 80),
        (120395, 80),
        (120447, 80),
        (929, 80),
        (120504, 80),
        (120562, 80),
        (120620, 80),
        (120678, 80),
        (120736, 80),
        (11426, 80),
        (5090, 80),
        (5229, 80),
        (42193, 80),
        (66197, 80),
        (119850, 113),
        (119902, 113),
        (119954, 113),
        (120006, 113),
        (120058, 113),
        (120110, 113),
        (120162, 113),
        (120214, 113),
        (120266, 113),
        (120318, 113),
        (120370, 113),
        (120422, 113),
        (120474, 113),
        (1307, 113),
        (1379, 113),
        (1382, 113),
        (8474, 81),
        (119824, 81),
        (119876, 81),
        (119928, 81),
        (119980, 81),
        (120032, 81),
        (120084, 81),
        (120188, 81),
        (120240, 81),
        (120292, 81),
        (120344, 81),
        (120396, 81),
        (120448, 81),
        (11605, 81),
        (119851, 114),
        (119903, 114),
        (119955, 114),
        (120007, 114),
        (120059, 114),
        (120111, 114),
        (120163, 114),
        (120215, 114),
        (120267, 114),
        (120319, 114),
        (120371, 114),
        (120423, 114),
        (120475, 114),
        (43847, 114),
        (43848, 114),
        (7462, 114),
        (11397, 114),
        (43905, 114),
        (119318, 82),
        (8475, 82),
        (8476, 82),
        (8477, 82),
        (119825, 82),
        (119877, 82),
        (119929, 82),
        (120033, 82),
        (120189, 82),
        (120241, 82),
        (120293, 82),
        (120345, 82),
        (120397, 82),
        (120449, 82),
        (422, 82),
        (5025, 82),
        (5074, 82),
        (66740, 82),
        (5511, 82),
        (42211, 82),
        (94005, 82),
        (65363, 115),
        (119852, 115),
        (119904, 115),
        (119956, 115),
        (120008, 115),
        (120060, 115),
        (120112, 115),
        (120164, 115),
        (120216, 115),
        (120268, 115),
        (120320, 115),
        (120372, 115),
        (120424, 115),
        (120476, 115),
        (42801, 115),
        (445, 115),
        (1109, 115),
        (43946, 115),
        (71873, 115),
        (66632, 115),
        (65331, 83),
        (119826, 83),
        (119878, 83),
        (119930, 83),
        (119982, 83),
        (120034, 83),
        (120086, 83),
        (120138, 83),
        (120190, 83),
        (120242, 83),
        (120294, 83),
        (120346, 83),
        (120398, 83),
        (120450, 83),
        (1029, 83),
        (1359, 83),
        (5077, 83),
        (5082, 83),
        (42210, 83),
        (94010, 83),
        (66198, 83),
        (66592, 83),
        (119853, 116),
        (119905, 116),
        (119957, 116),
        (120009, 116),
        (120061, 116),
        (120113, 116),
        (120165, 116),
        (120217, 116),
        (120269, 116),
        (120321, 116),
        (120373, 116),
        (120425, 116),
        (120477, 116),
        (8868, 84),
        (10201, 84),
        (128872, 84),
        (65332, 84),
        (119827, 84),
        (119879, 84),
        (119931, 84),
        (119983, 84),
        (120035, 84),
        (120087, 84),
        (120139, 84),
        (120191, 84),
        (120243, 84),
        (120295, 84),
        (120347, 84),
        (120399, 84),
        (120451, 84),
        (932, 84),
        (120507, 84),
        (120565, 84),
        (120623, 84),
        (120681, 84),
        (120739, 84),
        (11430, 84),
        (5026, 84),
        (42196, 84),
        (93962, 84),
        (71868, 84),
        (66199, 84),
        (66225, 84),
        (66325, 84),
        (119854, 117),
        (119906, 117),
        (119958, 117),
        (120010, 117),
        (120062, 117),
        (120114, 117),
        (120166, 117),
        (120218, 117),
        (120270, 117),
        (120322, 117),
        (120374, 117),
        (120426, 117),
        (120478, 117),
        (42911, 117),
        (7452, 117),
        (43854, 117),
        (43858, 117),
        (651, 117),
        (965, 117),
        (120534, 117),
        (120592, 117),
        (120650, 117),
        (120708, 117),
        (120766, 117),
        (1405, 117),
        (66806, 117),
        (71896, 117),
        (8746, 85),
        (8899, 85),
        (119828, 85),
        (119880, 85),
        (119932, 85),
        (119984, 85),
        (120036, 85),
        (120088, 85),
        (120140, 85),
        (120192, 85),
        (120244, 85),
        (120296, 85),
        (120348, 85),
        (120400, 85),
        (120452, 85),
        (1357, 85),
        (4608, 85),
        (66766, 85),
        (5196, 85),
        (42228, 85),
        (94018, 85),
        (71864, 85),
        (8744, 118),
        (8897, 118),
        (65366, 118),
        (8564, 118),
        (119855, 118),
        (119907, 118),
        (119959, 118),
        (120011, 118),
        (120063, 118),
        (120115, 118),
        (120167, 118),
        (120219, 118),
        (120271, 118),
        (120323, 118),
        (120375, 118),
        (120427, 118),
        (120479, 118),
        (7456, 118),
        (957, 118),
        (120526, 118),
        (120584, 118),
        (120642, 118),
        (120700, 118),
        (120758, 118),
        (1141, 118),
        (1496, 118),
        (71430, 118),
        (43945, 118),
        (71872, 118),
        (119309, 86),
        (1639, 86),
        (1783, 86),
        (8548, 86),
        (119829, 86),
        (119881, 86),
        (119933, 86),
        (119985, 86),
        (120037, 86),
        (120089, 86),
        (120141, 86),
        (120193, 86),
        (120245, 86),
        (120297, 86),
        (120349, 86),
        (120401, 86),
        (120453, 86),
        (1140, 86),
        (11576, 86),
        (5081, 86),
        (5167, 86),
        (42719, 86),
        (42214, 86),
        (93960, 86),
        (71840, 86),
        (66845, 86),
        (623, 119),
        (119856, 119),
        (119908, 119),
        (119960, 119),
        (120012, 119),
        (120064, 119),
        (120116, 119),
        (120168, 119),
        (120220, 119),
        (120272, 119),
        (120324, 119),
        (120376, 119),
        (120428, 119),
        (120480, 119),
        (7457, 119),
        (1121, 119),
        (1309, 119),
        (1377, 119),
        (71434, 119),
        (71438, 119),
        (71439, 119),
        (43907, 119),
        (71919, 87),
        (71910, 87),
        (119830, 87),
        (119882, 87),
        (119934, 87),
        (119986, 87),
        (120038, 87),
        (120090, 87),
        (120142, 87),
        (120194, 87),
        (120246, 87),
        (120298, 87),
        (120350, 87),
        (120402, 87),
        (120454, 87),
        (1308, 87),
        (5043, 87),
        (5076, 87),
        (42218, 87),
        (5742, 120),
        (10539, 120),
        (10540, 120),
        (10799, 120),
        (65368, 120),
        (8569, 120),
        (119857, 120),
        (119909, 120),
        (119961, 120),
        (120013, 120),
        (120065, 120),
        (120117, 120),
        (120169, 120),
        (120221, 120),
        (120273, 120),
        (120325, 120),
        (120377, 120),
        (120429, 120),
        (120481, 120),
        (5441, 120),
        (5501, 120),
        (5741, 88),
        (9587, 88),
        (66338, 88),
        (71916, 88),
        (65336, 88),
        (8553, 88),
        (119831, 88),
        (119883, 88),
        (119935, 88),
        (119987, 88),
        (120039, 88),
        (120091, 88),
        (120143, 88),
        (120195, 88),
        (120247, 88),
        (120299, 88),
        (120351, 88),
        (120403, 88),
        (120455, 88),
        (42931, 88),
        (935, 88),
        (120510, 88),
        (120568, 88),
        (120626, 88),
        (120684, 88),
        (120742, 88),
        (11436, 88),
        (11613, 88),
        (5815, 88),
        (42219, 88),
        (66192, 88),
        (66228, 88),
        (66327, 88),
        (66855, 88),
        (611, 121),
        (7564, 121),
        (65369, 121),
        (119858, 121),
        (119910, 121),
        (119962, 121),
        (120014, 121),
        (120066, 121),
        (120118, 121),
        (120170, 121),
        (120222, 121),
        (120274, 121),
        (120326, 121),
        (120378, 121),
        (120430, 121),
        (120482, 121),
        (655, 121),
        (7935, 121),
        (43866, 121),
        (947, 121),
        (8509, 121),
        (120516, 121),
        (120574, 121),
        (120632, 121),
        (120690, 121),
        (120748, 121),
        (1199, 121),
        (4327, 121),
        (71900, 121),
        (65337, 89),
        (119832, 89),
        (119884, 89),
        (119936, 89),
        (119988, 89),
        (120040, 89),
        (120092, 89),
        (120144, 89),
        (120196, 89),
        (120248, 89),
        (120300, 89),
        (120352, 89),
        (120404, 89),
        (120456, 89),
        (933, 89),
        (978, 89),
        (120508, 89),
        (120566, 89),
        (120624, 89),
        (120682, 89),
        (120740, 89),
        (11432, 89),
        (1198, 89),
        (5033, 89),
        (5053, 89),
        (42220, 89),
        (94019, 89),
        (71844, 89),
        (66226, 89),
        (119859, 122),
        (119911, 122),
        (119963, 122),
        (120015, 122),
        (120067, 122),
        (120119, 122),
        (120171, 122),
        (120223, 122),
        (120275, 122),
        (120327, 122),
        (120379, 122),
        (120431, 122),
        (120483, 122),
        (7458, 122),
        (43923, 122),
        (71876, 122),
        (66293, 90),
        (71909, 90),
        (65338, 90),
        (8484, 90),
        (8488, 90),
        (119833, 90),
        (119885, 90),
        (119937, 90),
        (119989, 90),
        (120041, 90),
        (120197, 90),
        (120249, 90),
        (120301, 90),
        (120353, 90),
        (120405, 90),
        (120457, 90),
        (918, 90),
        (120493, 90),
        (120551, 90),
        (120609, 90),
        (120667, 90),
        (120725, 90),
        (5059, 90),
        (42204, 90),
        (71849, 90),
        (65282, 34),
        (65284, 36),
        (65285, 37),
        (65286, 38),
        (65290, 42),
        (65291, 43),
        (65294, 46),
        (65295, 47),
        (65296, 48),
        (65297, 49),
        (65298, 50),
        (65299, 51),
        (65300, 52),
        (65301, 53),
        (65302, 54),
        (65303, 55),
        (65304, 56),
        (65305, 57),
        (65308, 60),
        (65309, 61),
        (65310, 62),
        (65312, 64),
        (65316, 68),
        (65318, 70),
        (65319, 71),
        (65324, 76),
        (65329, 81),
        (65330, 82),
        (65333, 85),
        (65334, 86),
        (65335, 87),
        (65343, 95),
        (65346, 98),
        (65348, 100),
        (65350, 102),
        (65355, 107),
        (65357, 109),
        (65358, 110),
        (65361, 113),
        (65362, 114),
        (65364, 116),
        (65365, 117),
        (65367, 119),
        (65370, 122),
        (65371, 123),
        (65373, 125),
        (160, 32),
        (8211, 45),
        (65374, 126),
        (65306, 58),
        (65281, 33),
        (8216, 96),
        (8217, 96),
        (8245, 96),
        (180, 96),
        (12494, 47),
        (1047, 51),
        (1073, 54),
        (1072, 97),
        (1040, 65),
        (1068, 98),
        (1042, 66),
        (1089, 99),
        (1057, 67),
        (1077, 101),
        (1045, 69),
        (1053, 72),
        (305, 105),
        (1050, 75),
        (921, 73),
        (1052, 77),
        (1086, 111),
        (1054, 79),
        (1009, 112),
        (1088, 112),
        (1056, 80),
        (1075, 114),
        (1058, 84),
        (215, 120),
        (1093, 120),
        (1061, 88),
        (1091, 121),
        (1059, 89),
        (65283, 35),
        (65288, 40),
        (65289, 41),
        (65292, 44),
        (65307, 59),
        (65311, 63),
    ])
});

pub fn ambiguous_unicode_character(
    locator: &SourceCodeLocator,
    start: &Location,
    end: &Location,
    is_docstring: bool,
    fix: bool,
) -> Vec<Check> {
    let mut checks = vec![];

    let text = locator.slice_source_code_range(&Range {
        location: *start,
        end_location: *end,
    });

    let mut col_offset = 0;
    let mut row_offset = 0;
    for current_char in text.chars() {
        // Search for confusing characters.
        if let Some(representant) = CONFUSABLES.get(&(current_char as u32)) {
            if let Some(representant) = char::from_u32(*representant) {
                let location = if row_offset == 0 {
                    Location::new(start.row() + row_offset, start.column() + col_offset)
                } else {
                    Location::new(start.row() + row_offset, col_offset)
                };
                let end_location = Location::new(location.row(), location.column() + 1);
                let mut check = Check::new(
                    if is_docstring {
                        CheckKind::AmbiguousUnicodeCharacterDocstring(current_char, representant)
                    } else {
                        CheckKind::AmbiguousUnicodeCharacterString(current_char, representant)
                    },
                    Range {
                        location,
                        end_location,
                    },
                );
                if fix {
                    check.amend(Fix::replacement(
                        representant.to_string(),
                        location,
                        end_location,
                    ));
                }
                checks.push(check);
            }
        }

        // Track the offset from the start position as we iterate over the body.
        if current_char == '\n' {
            col_offset = 0;
            row_offset += 1;
        } else {
            col_offset += 1;
        }
    }

    checks
}
