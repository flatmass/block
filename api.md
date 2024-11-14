# Программный интерфейс узла

## Структуры данных

### UpdateResponse

Стандартный ответ метода, генерирующего транзакцию в системе.

Ответ в случае успешной отправки транзакции:

* Код ответа - 200 (HTTP Success)
* Тело ответа в формате JSON c полем `data`:
* `data`
    * `tx_hash`: `Hash` - хэш сформированной транзакции, является её идентификатором

Ответ в случае ошибки при обработке запроса:

* Код ответа - 400 (HTTP Bad Request) или иной
* Тело ответа в формате JSON с полем errors:
    * `errors` - массив строк с ошибками выполнения запроса

### ObjectIdentity

Идентификатор ОИС

* `class`: `string` - тип ОИС:
    * `trademark` - товарный знак
    * `wellknown_trademark` - общеизвестный товарный знак
    * `appellation_of_origin` - НМПТ
    * `appellation_of_origin_rights` - ПНМПТ
    * `pharmaceutical` - фарм. препараты
    * `invention` - изобретения
    * `utility_model` - полезные модели
    * `industrial_model` - промышленные образцы
* `reg_number`: `string` - регистрационный номер

Передается в виде строки в формате `{class}::{reg_number}`

Примеры: `trademark::123451`

### MemberIdentity

Идентификатор участника

* `type`: `string` - тип участника
    * `ogrn` - ОГРН
    * `ogrnip` - ОГРНИП
    * `snils` - СНИЛС

Передается в виде строки в формате `{type}::{id}`.\
Для юридического лица должно быть задано поле `ogrn`, для физического лица - `snils`.

Примеры: `ogrn::1053600591197`, `snils::02583651862`

### Hash

Строка, содержащая хэш

Пример:
`d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad`

### Application

Данные заявления.

* `tx_hash`: `Hash` - хэш заявления
* `requestor`: `MemberIdentity` - владелец заявления
* `application`: `Hash` - хэш заявления
* `documents`: array of `Hash` - документы
* `status`: `string` - статус заявления

### LotInfo

* `name`: `string` - краткое наименование лота
* `desc`: `string` - описание лота
* `price`: `number` - начальная цена лота
* `sale_type`: `SaleType` - схема продажи
* `opening_time`: `DateTime` - время публикации лота
* `closing_time`: `DateTime` - время автоматического закрытия лота

### LotInfoWithObjects

* `name`: `string` - краткое наименование лота
* `desc`: `string` - описание лота
* `price`: `number` - текущая цена на лот (макс.)
* `sale_type`: `SaleType` - схема продажи
* `opening_time`: `DateTime` - время публикации лота RFC3339
* `closing_time`: `DateTime` - время автоматического закрытия лота в формате RFC3339
* `objects`: array of `ObjectIdentity` - массив идентификаторов ОИС
* `status`: [LotStatus](#lotstatus) - текущий статус лота

### LotStatus

Статус лота:

* `new` = 0 - after creation
* `rejected` = 1 - after internal verification (bad)
* `verified` = 2 - after internal verification (good)
* `completed` = 3 - after lot timeout
* `executed` = 4 - after publishing bids
* `closed` = 5 - after lot execution while object is updating
* `undefined` = 255 - something has been changed with objects while lot was opened

### SaleType

Тип продажи:

* `auction` = 1
* `private_sale` = 2

### OwnershipInfo

Информация о владении. Может передаваться в структурированном и неструктурированном виде. В
неструктурированном виде все поля, кроме `representation` необязательны.

Структурированный вид:

* `representation`: `string = "structured"` - представление информации
* `rightholder`: `string` - правообладатель
* `contract_type`: `ContractType` - тип контракта
* `exclusive`: `bool` - владение эксклюзивным правом
* `can_distribute`: `Distribution` - возможность распространения
* `location`: array of `Location` - территориальное ограничение прав
* `classifiers`: array of `Classifier` - ограничение права по классификаторам товаров и услуг
* `starting_time`: `DateTime` - дата начала владения
* `expiration_time`: `DateTime` or `null` - дата окончания владения

Неструктурированный вид:

* `representation`: `string = "unstructured"` - представление информации
* `data`: `string` or `null` - неструрированная информация о владении
* `rightholder`: `string` or `null` - правообладатель
* `exclusive`: `bool` or `null` - владение эксклюзивным правом

Примеры:

В структурированном виде
```json
{
    "representation": "structured",
    "rightholder": "ogrn::5068681643685",
    "contract_type": "license",
    "exclusive": true,
    "can_distribute": "able",
    "location": [ "oktmo::45379000" ],
    "classifiers": [ "mktu::8" ],
    "starting_time": "2020-06-01T00:00:00Z",
    "expiration_time": "2021-06-01T00:00:00Z"
}
```

В неструктурированном виде. Если все поля отсутствуют, запись просто указывает на наличие
информации о владении и влияет на работу автоматических проверок контракта.
```json
{
    "representation": "unstructured"
}
```

```json
{
    "representation": "unstructured",
    "rightholder": "ogrn::5068681643685",
    "exclusive": true
}
```

### Distribution

Возможность распространения:

* `able` = 1
* `with_written_permission` = 2
* `unable` = 3

### ObjectOwnership

* `object`: `ObjectIdentity`
* `contract_term`: `Term` or `null (forever)`
* `exclusive`: `bool`
* `can_distribute`: `Distribution`
* `location`: array of `Location` or `null (all)`
* `classifiers`: array of `Classifier` or `null (all)`

### Conditions

Условия передачи прав

* `contract_type`: `ContractType`
* `objects`: array of `ObjectOwnership` - список объектов и условий
* `payment_conditions`: `string` - условия оплаты
* `payment_comment`: `string` or `null` - другие условия и/или виды оплаты
* `termination_conditions`: array of `string` - условия прекращения действия сделки
* `contract_extras`: array of `string` - дополнительные условия контракта

### ContractType

Тип контракта. Передаётся в виде строки, отображается в транзакциях в виде числа.

Тип контракта:

* `undefined` = 0
* `license` = 1
* `sublicense` = 2
* `concession_agreement` = 3
* `subconcession_agreement` = 4
* `expropriation` = 5

### ContractStatus

Текущий статус контракта. Имеет вид строки.

Статус контракта:

* `new`
* `draft`
* `confirmed`
* `refused`
* `signed`
* `registering`
* `awaiting_user_action`
* `approved`
* `rejected`

### Location

Имеет следующие форматы:
* ОКТМО (с опциональным дополнительным постфиксом)
* Свободный

В виде ОКТМО должен быть префикс `oktmo::`, такой формат местонахождения может быть верифицирован
автоматически. При наличии дополнительного постфикса с уточнением формат может быть верифицирован
только частично.

Свободный формат требует ручной верификации.

Пример: `oktmo::45379000`
Пример: `oktmo::45379000::Проспект Мира`

### Classifier

Классификатор товаров и услуг

* `registry`: `string` - наименование справочника классификации:
    * `mktu` - МКТУ, Международная классификация товаров и услуг
    * `mpk` - МПК, Международная патентная классификация
    * `spk` - СПК, Совместная патентная классификация
    * `mkpo` - МКПО, Международная классификация промышленных образцов
* `value`: `string` - значение

Передается в виде строки в формате `{registry}::{value}`

Примеры: `mktu::8`

### DateTime

Структура описания точного времени совершения транзакции. Время возвращается в соответствии с
RFC3339.

Пример: `1996-12-19T16:39:57-08:00`

### Term

Структура описания срока времени

* `specification`: `string`
    * `for` - на определённое количество дней/месяцев/лет
    * `to` - до указанной даты (не включительно)
    * `until` - по указанную дату (включительно)
    * `forever` - на срок действия исключительного права/патента
* `duration`: `string` if `specification` is `for` else `null`
* `date`: `DateTime` if `specification` is `to` or `until` else `null`

`duration` поддерживается в формате `"{months}:{days}"`. Годы, месяцы и кварталы указываются в
суммарном кол-ве месяцев. Дни и недели указываются в суммарном кол-ве дней, без учета месяцев
(остаток). `1:7` - месяц + неделя, `3:0` - квартал, `12:0`- год, `60:0` - пять лет

Примеры:
```json
{
  "specification": "for",
  "duration": "1:7"
}
```
```json
{
  "specification": "to",
  "date": "2025-20-16T14:09:00-08:00"
}
```
```json
{
  "specification": "forever"
}
```

### AttachmentType

Тип документа. Имеет вид строки.

Возможные типы документов:

* `deed` - файл договора
* `application` - файл заявления
* `other` - другое

### CheckKey

Тип автоматической проверки контракта.

Проверки на чтение и запись имеют вид одного из следующих значений и передается в виде строки:

* `tax_payment_info_added` - см. 9
* `blacklist` - см. 8
* `seller_data_valid` - см. 2.1
* `duration_valid` - см. 7
* `usecases_match` - см. 4.6
* `registered_changes` - см. 5.2
* `public_expropriation_offer` - см. 5.3

Проверки на чтение имеют вид одного из следующих значений:

* `documents_match_condition`
* `can_sell`
* `can_buy`
* `location_valid`
* `object_duplicates`
* `objects_sellable`
* `contains_trademark`
* `contains_appellation_of_origin`

### CheckResult

Статус проверки.

Имеет одно из следующих значений и передается в виде строки:

* `ok` - успешный результат прохождения проверки
* `unknown` - требуется проверка экспертом
* `error` - неуспешный результат прохождения проверки

### CheckInfo

Содержит информацию о статусе автоматической проверки контракта.

Структура:

* `result`: `CheckResult` - статус проверки.
* `description`: `string` - содержит расширенную информацию о статусе проверки.


----------------------------------------------------------------------------------------------------

## API

Поле **ИНТЕРФЕЙС** определяет публичный(`public`)/приватный(`private`) интерфейс подключения к узлу. При
значении `public` запрос отправляется на публичный IP, при значении `private` - на приватный.

### Участники. Добавить участника

Добавляет участника взаимодействия, выполняется после добавлению партнера в сеть узлов блокчейн решения.

### ContractInfo

Содержит информацию о контракте

Структура:

* `buyer`: `MemberIdentity` - Покупатель
* `seller`: `MemberIdentity` - Продавец
* `price`: `number` - стоимость в копейках
* `conditions`: `ConditionsInfo` - условия совершения сделки
* `status`: `ContractStatus` - статус контракта
* `deed_tx_hash`: `Hash` or `null` - хэш транзакции добавления файла договора
* `application_tx_hash`: `Hash` or `null` - хэш транзакции добавления файла уведомления
* `stored_docs`: array of `Hash` - Массив транзакций добавления документов (договор и уведомление прикрепляються
  отдельным методом)
* `reference_number`: `string` or `null` - номер дела, которое по нашему заявлению завёл ФИПС

----------------------------------------------------------------------------------------------------

**МЕТОД**: `POST`

**АДРЕС**: `/members`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `member`: `MemberIdentity` - идентификатор участника
* `node`: `string` - имя узла, должно совпадать с именем узла, указанным при добавлении сертификата узла

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AddMember](transactions.md#addmember) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[Too long param](errors.md#too-long-param)

----------------------------------------------------------------------------------------------------

### ОИС. Ввести в оборот

Является ответом Системы на "ОИС. Запрос на ввод в оборот".

**МЕТОД**: `POST`

**АДРЕС**: `/objects`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `owner`: `text/plain` `MemberIdentity` - идентификатор владельца ОИС
* `object`: `text/plain` `ObjectIdentity` - идентификатор ОИС
* `data`: `TODO` `string` - публичная информация охранного документа в формате xml
* `ownership`: `application/json` array of `OwnershipInfo` - информация о владении в формате JSON

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**:  [AddObject](transactions.md#addobject) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad object format](errors.md#bad-object-format),
[Bad member format](errors.md#bad-member-format),
[Bad classifier format](errors.md#bad-classifier-format),
[Bad contract type format](errors.md#bad-contract-type-format),
[Bad location](errors.md#bad-location)

### ОИС. Изменить ОИС

Вызывается Системой при изменениях в ОИС

**МЕТОД**: `PUT`

**АДРЕС**: `/objects`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `owner`: `text/plain` `MemberIdentity` - идентификатор владельца ОИС
* `object`: `text/plain` `ObjectIdentity` - идентификатор ОИС
* `data`: `TODO` `string` - публичная информация охранного документа в формате xml
* `ownership`: `application/json` array of `OwnershipInfo` - информация о владении в формате JSON

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [UpdateObject](transactions.md#updateobject) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad object format](errors.md#bad-object-format),
[Bad member format](errors.md#bad-member-format),
[Bad classifier format](errors.md#bad-classifier-format),
[Bad contract type format](errors.md#bad-contract-type-format),
[Bad location](errors.md#bad-location)

### ОИС. Запрос на ввод в оборот ОИС

Запрос на размещение в Системе ОИС. Запрос может быть отправлен от любого участника сети.

**МЕТОД**: `POST`

**АДРЕС**: `/objects/request`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `object`: `ObjectIdentity` - идентификатор ОИС

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AddObjectRequest](transactions.md#addobjectrequest) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad object format](errors.md#bad-object-format),
[Bad member format](errors.md#bad-member-format)

### ОИС. Запрос на ввод в оборот группы ОИС

Запрос на ввод в оборот всех ОИС участника.

**МЕТОД**: `POST`

**АДРЕС**: `/objects/request/member`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AddObjectGroupRequest](transactions.md#addobjectgrouprequest) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format)

### ОИС. Получить ОИС

**МЕТОД**: `GET`

**АДРЕС**: `/objects`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `object`: `ObjectIdentity` - идентификатор ОИС

**ОТВЕТ**:

* `data`
  * `object_info` - публичная информация охранного документа в формате xml

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad object format](errors.md#bad-object-format),
[No object](errors.md#no-object)

### ОИС. Получить список ОИС для участника

**МЕТОД**: `GET`

**АДРЕС**: `/objects`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `owner`: `MemberIdentity` - идентификатор участника

**ОТВЕТ**:

* `data`
    * `objects`: array of `ObjectIdentity` - массив идентификаторов ОИС

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad member format](errors.md#bad-member-format)

### ОИС. Получить историю изменений ОИС

**МЕТОД**: `GET`

**АДРЕС**: `/objects/history`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `object`: `ObjectIdentity` - идентификатор ОИС

**ОТВЕТ**:

* `data`
    * `tx_hashes`: array of `string` - массив идентификаторов транзакции создания ОИС и транзакций изменения данных ОИС

**ОШИБКИ**:
[Bad object format](errors.md#bad-object-format),
[Bad JSON](errors.md#bad-json)

----------------------------------------------------------------------------------------------------

### Лоты. Добавить лот

Открыть лот может только владелец прав на ОИС участвующие в лоте. Лот будет добавлен в статусе
"new", и должен пройти проверку в ФИПС, после чего статус будет измененён на "rejected" или
"verified" (ожидаем транзакцию "Лоты. Сменить статус лота").

**МЕТОД**: `POST`

**АДРЕС**: `/lots`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `lot`: `LotInfo` - информация о лоте
* `ownership`: `Conditions` - условия совершения сделки

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [OpenLot](transactions.md#openlot) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad object format](errors.md#bad-object-format),
[Bad member format](errors.md#bad-member-format),
[Bad classifier format](errors.md#bad-classifier-format),
[Bad contract type format](errors.md#bad-contract-type-format),
[Bad location](errors.md#bad-location),
[Bad term format](errors.md#bad-term-format),
[Bad sale type format](errors.md#bad-sale-type-format)

### Лоты. Сменить статус лота

**МЕТОД**: `PUT`

**АДРЕС**: `/lots`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота
* `status`: `string` - статус лота (`rejected`, `verified`)

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [EditLotStatus](transactions.md#editlotstatus) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad state](errors.md#bad-state)

### Получить список предложения по лоту

**МЕТОД**: `GET`

**АДРЕС**: `/lots/bids`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота

**ОТВЕТ**:

* `data`
    * `bids`: array of `number` - массив предложений по лоту, стоимость в рублях

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Crypto error](errors.md#crypto-error)

### Получить список транзакций предложений по лоту

**МЕТОД**: `GET`

**АДРЕС**: `/lots/bids/transactions`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота

**ОТВЕТ**:

* `data`
    * `tx_hashes`: array of `Hash` - массив транзакций предложений по лоту

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Crypto error](errors.md#crypto-error)

### Лоты. Опубликовать предложения по лоту

**МЕТОД**: `POST`

**АДРЕС**: `/lots/bids/publish`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота
* `bids`: array of `number` - массив предложенных цен (только успешно выполненных транзакций, которые еще не были
  опубликованы)

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [PublishBids](transactions.md#publishbids) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json)

### Лоты. Исполнить лот

Выбирает максимальную цену и закрывает лот

**МЕТОД**: `POST`

**АДРЕС**: `/lots/execute`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ExecuteLot](transactions.md#executelot) (public)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json)

### Лоты. Снять лот с торгов

Закрыть лот может только владелец прав на ОИС участвующие в лоте.

**МЕТОД**: `DELETE`

**АДРЕС**: `/lots`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота
* `requestor`: `MemberIdentity` - идентификатор запрашивающего

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [CloseLot](transactions.md#closelot) (public)

**ОШИБКИ**:
[Bad member format](errors.md#bad-member-format),
[Bad JSON](errors.md#bad-json)

### Лоты. Предложить цену за лот

**МЕТОД**: `POST`

**АДРЕС**: `/lots/bids`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота
* `value` - `number` предложение цены в копейках

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AddBid](transactions.md#addbid) (private)

**ОШИБКИ**:
[Bad member format](errors.md#bad-member-format),
[Bad JSON](errors.md#bad-json)

### Лоты. Получить список открытых лотов

**МЕТОД**: `GET`

**АДРЕС**: `/lots`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `member` (опционально): `MemberIdentity` - идентификатор участника

**ОТВЕТ**:

* `data`
    * `tx_hashes`: array of `Hash` - массив идентификаторов транзакций открытия лотов или лотов участника

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad member format](errors.md#bad-member-format)

### Лоты. Получить информацию по лоту

**МЕТОД**: `GET`

**АДРЕС**: `/lots`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `lot_tx_hash`: `Hash` - хэш транзакции открытия лота. Параметр `member`
  из [запроса](#Лоты.-Получить-список-открытых-лотов) имеет больший приоритет при наличии обоих параметров в запросе.

**ОТВЕТ**:
* `data`
    * `lot`: [LotInfoWithObjects](#lotinfowithobjects) - содержит информацию по лоту

**ОШИБКИ**:
[Crypto error](errors.md#crypto-error),
[Bad lot status](errors.md#bad-lot-status),
[No lot](errors.md#no-lot),
[Bad state](errors.md#bad-state)

### Лоты. Продлить срок публикации лота

**МЕТОД**: `POST`

**АДРЕС**: `/lots/extend`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота
* `new_expiration`: `DateTime` - новое время автоматического закрытия лота. Должно быть позже текущего

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ExtendLotPeriod](transactions.md#extendlotperiod)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json)

----------------------------------------------------------------------------------------------------

## Контракты

### Контракты. Приобрести группу ОИС одного участника

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/purchase_offer`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `rightholder`: `MemberIdentity` - идентификатор владельца списка ОИС
* `price`: `number` - стоимость в копейках
* `conditions`: `Conditions` - условия приобретения

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [PurchaseOffer](transactions.md#purchaseoffer) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad object format](errors.md#bad-object-format),
[Bad classifier format](errors.md#bad-classifier-format),
[Bad location](errors.md#bad-location),
[Bad contract type format](errors.md#bad-contract-type-format),
[Bad term format](errors.md#bad-term-format),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Приобрести лот

В случае продажи лота по фиксированной цене запрос доступен всем участникам, в случае аукциона запрос должен выполняться внутренним сервисом от лица участника, выигравшего торги за лот.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/acquire_lot`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `lot_tx_hash`: `Hash` - хэш транзакции выставления лота

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AcquireLot](transactions.md#acquirelot) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[Bad stored member](errors.md#bad-stored-member),
[No transaction](errors.md#no-transaction)

### Контракты. Изменение условий контракта

**МЕТОД**: `PUT`

**АДРЕС**: `/contracts`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции добавления контракта
* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `price`: `number` - стоимость в копейках
* `conditions`: `Conditions` - условия приобретения

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [UpdateContract](transactions.md#updatecontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[Bad contract type format](errors.md#bad-contract-type-format),
[Bad object format](errors.md#bad-object-format),
[Bad term format](errors.md#bad-term-format),
[Bad location](errors.md#bad-location),
[Bad classifier format](errors.md#bad-classifier-format)

### Контракты. Перевод контракта в Draft

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/draft`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `doc_tx_hashes`: array of `Hash` - хэши документов контракта
* `deed_tx_hash`: `hash` - Хэш транзакции добавления файла договора
* `application_tx_hash`:`hash` - Хэш транзакции добавления файла заявления

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [DraftContract](transactions.md#draftcontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Добавление документа контракта

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/documents`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `requestor`: `text/plain` `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `text/plain` `Hash` - хэш транзакции добавления контракта
* `name`: `text/plain` - наименование документа
* `file`: `TODO` - файл документа (договор, заявление)
* `file_type`: `text/plain` `AttachmentType` - тип документа (договор, заявление, другое)

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AttachContractFile](transactions.md#attachcontractfile) (public)

**ОШИБКИ**:
[Bad member format](errors.md#bad-member-format),
[Bad UTF-8](errors.md#bad-utf-8),
[Empty param](errors.md#empty-param),
[Crypto error](errors.md#crypto-error),
[No param](errors.md#no-param),
[Bad file type](errors.md#bad-file-type),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Удаление документов контракта

**МЕТОД**: `DELETE`

**АДРЕС**: `/contracts/documents`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `doc_tx_hashes`: array of `Hash` - хэши документов, которые нужно удалить

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [DeleteContractFile](transactions.md#deletecontractfile) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Отказ от контракта

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/refuse`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `reason`: `string` or `null` - причина отказа

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [RefuseContract](transactions.md#refusecontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Подтверждение контракта участником

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/confirm`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `deed_tx_hash`: `Hash` - хэш подтверждаемого документа договора
* `application_tx_hash`: `Hash` - хэш подтверждаемого документа заявления
* `doc_tx_hashes`: array of `Hash` - хэши подтверждаемых документов

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ConfirmContract](transactions.md#confirmcontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Добавление информации об оплате пошлины.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/tax`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `payment_number`: `string` - номер платёжки
* `payment_date`: `string` - дата платежа
* `amount`: `number` - сумма в копейках

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ConfirmContract](transactions.md#confirmcontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Empty param](errors.md#empty-param),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Подписание договора и заявления контракта.

После того как документы контракта согласованы и контракт имеет статус Confirmed необходимо подписать контракт обеими сторонами.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/sign`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `application_sign`: `string` - открепленная подпись файла заявления, преобразованная в base64 формат
* `deed_sign`: `string` - открепленная подпись файла договора, преобразованная в base64 формат

**ОТВЕТ**: структура `UpdateResponse`

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member),
[Crypto error](errors.md#crypto-error)

### Контракты. Перевести контракт в статус регистрации.

Генерирует приватную транзакцию смены статуса контракта на Registering, только если контракт находится в состоянии Confirmed.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/register`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [RegisterContract](transactions.md#registercontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Перевести контракт в статус ожидания действий от пользователя.

Генерирует приватную транзакцию смены статуса контракта на AwaitingUserAction, только если контракт находится в состоянии Registering.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/await_user_action`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AwaitUserActionContract](transactions.md#awaituseractioncontract) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Перевод контракта в состояние Approved

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/approve`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `text/plain` `Hash` - хэш транзакции создания контракта
* `file`: `TODO` - файл уведомления
* `name`: `text/plain` `string` - наименование документа
* `sign`: `text/plain` `string` - открепленная подпись файла/документа, преобразованная в base64 формат

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ApproveContract](transactions.md#approvecontract) (private)

**ОШИБКИ**:
[Unexpected param value](errors.md#unexpected-param-value),
[Too long param](errors.md#too-long-param),
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad UTF-8](errors.md#bad-utf-8),
[Crypto error](errors.md#crypto-error)

### Контракты. Отклонение контракта внутренним сервисом, проверки не пройдены

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/reject`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `text/plain` `Hash` - хэш транзакции создания контракта
* `reason`: `text/plain` `string` - причина отклонения контракта, описание ошибки
* `file`: `TODO` - файл уведомления. Параметр опционален: необходим когда контракт в находиться статусе "Registering"
  или "AwaitingUserAction", не нужен когда контракт в статусе "New".
* `name`: `text/plain` `string` - наименование документа. Параметр опционален: необходим когда контракт в находиться
  статусе "Registering" или "AwaitingUserAction", не нужен когда контракт в статусе "New".
* `sign`: `text/plain` `string` - открепленная подпись файла/документа, преобразованная в base64 формат. Параметр
  опционален: необходим когда контракт в находиться статусе "Registering" или "AwaitingUserAction", не нужен когда
  контракт в статусе "New".

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [RejectContract](transactions.md#rejectcontract) (private)

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad UTF-8](errors.md#bad-utf-8),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member),
[Crypto error](errors.md#crypto-error)

### Контракты. Добавление статуса автоматических проверок контракта.

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/checks`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `checks`:
    * [`CheckKey`](api.md#checkkey): `CheckInfo` - тип проверки передается в качестве ключа, а результат проверки в
      качестве значения. В случае наличия повторяющихся типов проверок, предпочтение отдаётся последней.

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [SubmitChecks](transactions.md#submitchecks) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Получить список проверок

**МЕТОД**: `GET`

**АДРЕС**: `/contracts/checks`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**:

* `data`
    * `checks`:
        * [`CheckKey`](api.md#checkkey): `CheckInfo` - информация о проверках, сгруппированная по ключам.

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[Bad member format](errors.md#bad-member-format),
[Crypto error](errors.md#crypto-error),
[No contract](errors.md#no-contract),
[No permission](errors.md#no-permission),
[Internal bad struct](errors.md#internal-bad-struct)

### Контракты. Получить статус контракта.

**МЕТОД**: `GET`

**АДРЕС**: `/contracts/status`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**:

* `data`
    * `status`: `ContractStatus` - статус контракта

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[No contract](errors.md#no-contract),
[Internal bad struct](errors.md#internal-bad-struct),
[No permission](errors.md#no-permission),
[Crypto error](errors.md#crypto-error)

### Контракты. Получить условия контракта.

**МЕТОД**: `GET`

**АДРЕС**: `/contracts/conditions`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**:

* `data`
  * `conditions`: `ConditionsInfo` - условия сделки

**ОШИБКИ**:
[No param](errors.md#no-param),
[Empty param](errors.md#empty-param),
[No contract](errors.md#no-contract),
[Internal bad struct](errors.md#internal-bad-struct),
[Unexpected param value](errors.md#unexpected-param-value),
[Crypto error](errors.md#crypto-error)

----------------------------------------------------------------------------------------------------

### Документы. Добавить документ

Генерирует приватную транзакцию добавления документа.

Документ попадает в приватное хранилище и может быть указан при подаче запроса к ФИПС.

**МЕТОД**: `POST`

**АДРЕС**: `/documents`

**ТИП**: `multipart/form-data`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `text/plain` `MemberIdentity` - идентификатор запрашивающего
* `file`: `TODO` - файл документа (договор, заявление)
* `file_type`: `text/plain` `AttachmentType` - тип документа (договор, заявление, другое)
* `name`: `text/plain` `string` - наименование документа
* `members`: TODO array of `MemberIdentity` - участники, которым следует предоставить доступ к документу (документ
  передается на узел участников)

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AttachFile](transactions.md#attachfile) (private)

**ОШИБКИ**:
[Empty param](errors.md#empty-param),
[No param](errors.md#no-param),
[Bad UTF-8](errors.md#bad-utf-8),
[Bad member format](errors.md#bad-member-format),
[Too long param](errors.md#too-long-param),
[Unexpected param value](errors.md#unexpected-param-value),
[Bad file type](errors.md#bad-file-type),
[Bad stored member](errors.md#bad-stored-member)

### Документы. Добавить подписи документа

Генерирует приватную транзакцию добавления подписей к ранее загруженному документу. Подписи
верифицируются.

**МЕТОД**: `POST`

**АДРЕС**: `/documents/signs`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `sign`: `string` - открепленная подпись файла/документа, преобразованная в base64 формат
* `doc_tx_hash`: `Hash` - хэш транзакции с подписываемым документом

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [AddAttachmentSign](transactions.md#addattachmentsign) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[Crypto error](errors.md#crypto-error),
[Unexpected transaction type](errors.md#unexpected-transaction-type)

### Документы. Удалить группу документов с их подписями

Генерирует приватную транзакцию удаления группы документов.

**МЕТОД**: `DELETE`

**АДРЕС**: `/documents`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ**:

* `requestor`: `MemberIdentity` - идентификатор запрашивающего
* `doc_tx_hashes`: array of `Hash` - хэши транзакций прикрепления удаляемых документов

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [DeleteFiles](transactions.md#deletefiles) (private)

**ОШИБКИ**:
[Bad JSON](errors.md#bad-json),
[Bad member format](errors.md#bad-member-format),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Добавление к контракту номера дела.

Добавление к контракту номера дела, которое по нашему заявлению завёл ФИПС

**МЕТОД**: `POST`

**АДРЕС**: `/contracts/reference_number`

**ТИП**: `application/json`

**ИНТЕРФЕЙС**: `private`

**ПАРАМЕТРЫ**:

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта
* `reference_number`: `string` - номер дела, которое по нашему заявлению завёл ФИПС

**ОТВЕТ**: структура `UpdateResponse`

**ТРАНЗАКЦИИ**: [ContractReferenceNumber](transactions.md#contractreferencenumber) (private)

**ОШИБКИ**:
[Too long param](errors.md#too-long-param),
[Unexpected param value](errors.md#unexpected-param-value),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)

### Контракты. Получение информации по контракту.

**МЕТОД**: `GET`

**АДРЕС**: `/contracts`

**ТИП**: `none`

**ИНТЕРФЕЙС**: `public`

**ПАРАМЕТРЫ** (query):

* `contract_tx_hash`: `Hash` - хэш транзакции создания контракта

**ОТВЕТ**:

* `data`
    * `contract_info`: `ContractInfo` - содержит информацию по контракту

**ОШИБКИ**:
[Crypto error](errors.md#crypto-error),
[Empty param](errors.md#empty-param),
[No param](errors.md#no-param),
[No contract](errors.md#no-contract),
[Bad stored member](errors.md#bad-stored-member)


